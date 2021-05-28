//! SPI SD Card driver for K210 SPI SD Card Slot

#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

use k210_pac::{Peripherals, SPI0};
use k210_hal::prelude::*;
use k210_soc::{
        gpio,
        gpiohs,
        spi::{aitm, tmod, work_mode, frame_format, SPI, SPIExt, SPIImpl},
        fpioa::{self, io},
        sysctl,
        sleep::usleep,
};
use spin::Mutex;
use lazy_static::*;
use super::BlockDevice;
use core::convert::TryInto;

pub const SD_START_DATA_SINGLE_BLOCK_READ: u8 = 0xFE;

pub const SD_START_DATA_SINGLE_BLOCK_WRITE: u8 = 0xFE;

pub const SD_START_DATA_MULTIPLE_BLOCK_WRITE: u8 = 0xFC;

/// Sector length
pub const SEC_LEN: usize = 512;

/// Commands for SPI SD Cards
#[repr(u8)]
#[derive(Debug, Copy, Clone)]
#[allow(unused)]
pub enum CMD {
        CMD0 = 0,       //** Software reset */
        CMD8 = 8,       //** Check voltage range (SDC V2) */
        CMD9 = 9,       //** Read CSD register */
        CMD10 = 10,     //** Read CID register */
        CMD12 = 12,     //** Stop to read data */
        CMD16 = 16,     //** Change R/W block size */
        CMD17 = 17,     //** Read block */
        CMD18 = 18,     //** Read multiple blocks */
        ACMD23 = 23,    //** Number of blocks to erase (SDC) */
        CMD24 = 24,     //** Write a block */
        CMD25 = 25,     //** Write multiple blocks */
        ACMD41 = 41,    //** Initiate initialization process (SDC) */
        CMD55 = 55,     //** Leading command for ACMD* */
        CMD58 = 58,     //** Read OCR */
        CMD59 = 59,     //** Enable/disable CRC check */
}

/// Initialization error type
#[derive(Debug, Copy, Clone)]
pub enum InitError {
        CMDFailed(CMD, u8),
        CannotGetCardInfo,
}

/// Card Identification Data: CID Register
#[derive(Debug, Copy, Clone)]
pub struct SDCardCID {
    pub ManufacturerID: u8, /* ManufacturerID */
    pub OEM_AppliID: u16,   /* OEM/Application ID */
    pub ProdName1: u32,     /* Product Name part1 */
    pub ProdName2: u8,      /* Product Name part2*/
    pub ProdRev: u8,        /* Product Revision */
    pub ProdSN: u32,        /* Product Serial Number */
    pub Reserved1: u8,      /* Reserved1 */
    pub ManufactDate: u16,  /* Manufacturing Date */
    pub CID_CRC: u8,        /* CID CRC */
    pub Reserved2: u8,      /* always 1 */
}

/// Card Specific Data: CSD Register
#[derive(Debug, Copy, Clone)]
pub struct SDCardCSD {
    /// CSD structure
    pub CSDStruct: u8,       
    /// System specification version
    pub SysSpecVersion: u8,  
    /// Reserved
    pub Reserved1: u8,       
    /// Data read access-time 1
    pub TAAC: u8,            
    /// Data read access-time 2 in CLK cycles
    pub NSAC: u8,            
    /// Max. bus clock frequency
    pub MaxBusClkFrec: u8,   
    /// Card command classes
    pub CardComdClasses: u16,
    /// Max. read data block length
    pub RdBlockLen: u8,      
    /// Partial blocks for read allowed
    pub PartBlockRead: u8,   
    /// Write block misalignment
    pub WrBlockMisalign: u8, 
    /// Read block misalignment
    pub RdBlockMisalign: u8, 
    /// DSR implemented
    pub DSRImpl: u8,         
    /// Reserved
    pub Reserved2: u8,       
    /// Device Size
    pub DeviceSize: u32,     
    pub DeviceSizeMult: u8,
    /// Erase group size 
    pub EraseGrSize: u8,         
    /// Erase group size multiplier 
    pub EraseGrMul: u8,          
    /// Write protect group size 
    pub WrProtectGrSize: u8,     
    /// Write protect group enable 
    pub WrProtectGrEnable: u8,   
    /// Manufacturer default ECC 
    pub ManDeflECC: u8,          
    /// Write speed factor 
    pub WrSpeedFact: u8,         
    /// Max. write data block length 
    pub MaxWrBlockLen: u8,       
    /// Partial blocks for write allowed 
    pub WriteBlockPaPartial: u8, 
    /// Reserded 
    pub Reserved3: u8,           
    /// Content protection application 
    pub ContentProtectAppli: u8, 
    /// File format group 
    pub FileFormatGroup: u8,     
    /// Copy flag (OTP) 
    pub CopyFlag: u8,            
    /// Permanent write protection 
    pub PermWrProtect: u8,       
    /// Temporary write protection 
    pub TempWrProtect: u8,       
    /// File Format 
    pub FileFormat: u8,          
    /// ECC code 
    pub ECC: u8,                 
    /// CSD CRC 
    pub CSD_CRC: u8,             
    /// always 1
    pub Reserved4: u8,           
}

#[derive(Debug, Copy, Clone)]
pub struct SDCardInfo {
    pub SD_csd: SDCardCSD,
    pub SD_cid: SDCardCID,
    /// Card capacity
    pub CardCapacity: u64,
    pub CardBlockCnt: u64,
     /// Card Block Size
    pub CardBlockSize: u64,
}

/// Representation of a SD Card
struct SDCard0 {
        spi:            SPIImpl<SPI0>,
        spi_cs:         u32,
        cs_gpionum:     u8,
        byte_addr:      bool,
        info:           Option<SDCardInfo>,
}

impl SDCard0 {
        /// Low level initialize a SD Card
        fn lowlevel_init(&self) {
                gpiohs::set_direction(self.cs_gpionum, gpio::direction::OUTPUT);
                self.spi.set_clk_rate(200000);
        }

        /// Set CS pin to high
        fn CS_HIGH(&self) {
                gpiohs::set_pin(self.cs_gpionum, true);
        }

        /// Set CS pin to low
        fn CS_LOW(&self) {
                gpiohs::set_pin(self.cs_gpionum, false);
        }

        /// Change spi clk rate to 8000000 for a higher speed
        fn HIGH_SPEED_ENABLE(&self) {
                self.spi.set_clk_rate(8000000);
        }

        /// write raw data to SD Card
        fn write_data(&self, data: &[u8]) {
                self.spi.configure(
                    work_mode::MODE0,
                    frame_format::STANDARD,
                    8, /* data bits */
                    0, /* endian */
                    0, /*instruction length*/
                    0, /*address length*/
                    0, /*wait cycles*/
                    aitm::STANDARD,
                    tmod::TRANS,
                );
                self.spi.send_data(self.spi_cs, data);
        }

        /// read raw data from SD Card
        fn read_data(&self, data: &mut [u8]) {
                self.spi.configure(
                        work_mode::MODE0,
                        frame_format::STANDARD,
                        8, /* data bits */
                        0, /* endian */
                        0, /*instruction length*/
                        0, /*address length*/
                        0, /*wait cycles*/
                        aitm::STANDARD,
                        tmod::RECV,
                );
                self.spi.recv_data(self.spi_cs, data);
        }

        /// send commands to SD Card
        fn send_cmd(&self, cmd: CMD, arg: u32, crc: u8) {
                /* SD chip select low */
                self.CS_LOW();
                /* Send the Cmd bytes */
                self.write_data(&[
                        /* Construct byte 1 */
                        ((cmd as u8) | 0x40),
                        /* Construct byte 2 */
                        (arg >> 24) as u8,
                        /* Construct byte 3 */
                        ((arg >> 16) & 0xff) as u8,
                        /* Construct byte 4 */
                        ((arg >> 8) & 0xff) as u8,
                        /* Construct byte 5 */
                        (arg & 0xff) as u8,
                        /* Construct CRC: byte 6 */
                        crc,
                ]);
        }

        /// end SD Card command sequence
        fn end_cmd(&self) {
                /* SD chip select high */
                self.CS_HIGH();
                /* Send the cmd byte */
                self.write_data(&[0xff]);
        }

        /// get respond from SD Card
        fn get_response(&self) -> u8 {
                let result = &mut [0u8];
                let mut timeout = 0x0FFF;
                /* Check if response is got or a timeout is happen */
                while timeout != 0 {
                        self.read_data(result);
                        /* Right response got */
                        if result[0] != 0xFF {
                                return result[0];
                        }
                        timeout -= 1;
                }
                /* After time out */
                return 0xFF;
        }

        /// get SD Card CSD register
        fn get_csdregister(&self) -> Result<SDCardCSD, ()> {
                let mut csd_tab = [0u8; 18];
                /* Send CMD9 (CSD register) */
                self.send_cmd(CMD::CMD9, 0, 0);
                /* Wait for response in the R1 format (0x00 is no errors) */
                if self.get_response() != 0x00 {
                    self.end_cmd();
                    return Err(());
                }
                if self.get_response() != SD_START_DATA_SINGLE_BLOCK_READ {
                    self.end_cmd();
                    return Err(());
                }
                /* Store CSD register value on csd_tab */
                /* Get CRC bytes (not really needed by us, but required by SD) */
                self.read_data(&mut csd_tab);
                self.end_cmd();
                /* see also: https://cdn-shop.adafruit.com/datasheets/TS16GUSDHC6.pdf */
                if csd_tab[0] >> 6 == 0 {
                        return Ok(SDCardCSD{
                                CSDStruct: 0,
                                SysSpecVersion: 0,
                                Reserved1: 0,
                                TAAC: 0,
                                NSAC: 0,
                                MaxBusClkFrec: 0,
                                CardComdClasses: 0,
                                RdBlockLen: csd_tab[5] & 0x0F,
                                PartBlockRead: 0,
                                WrBlockMisalign: 0,
                                RdBlockMisalign: 0,
                                DSRImpl: 0,
                                Reserved2: 0,
                                DeviceSize: (((csd_tab[6] & 0x3) as u32) << 10)
                                        | ((csd_tab[7] as u32) << 2) 
                                        | ((csd_tab[8] >> 6) as u32),
                                DeviceSizeMult: ((csd_tab[9] & 03) << 1) 
                                        | (csd_tab[10] >> 7),
                                EraseGrSize: 0,
                                EraseGrMul: 0,
                                WrProtectGrSize: 0,
                                WrProtectGrEnable: 0,
                                ManDeflECC: 0,
                                WrSpeedFact: 0,
                                MaxWrBlockLen: 0,
                                WriteBlockPaPartial: 0,
                                Reserved3: 0,
                                ContentProtectAppli: 0,
                                FileFormatGroup: 0,
                                CopyFlag: 0,
                                PermWrProtect: 0,
                                TempWrProtect: 0,
                                FileFormat: 0,
                                ECC: 0,
                                CSD_CRC: 0,
                                Reserved4: 10
                        })
                } else {
                        return Ok(SDCardCSD {
                                /* Byte 0 */
                                CSDStruct: (csd_tab[0] & 0xC0) >> 6,
                                SysSpecVersion: (csd_tab[0] & 0x3C) >> 2,
                                Reserved1: csd_tab[0] & 0x03,
                                /* Byte 1 */
                                TAAC: csd_tab[1],
                                /* Byte 2 */
                                NSAC: csd_tab[2],
                                /* Byte 3 */
                                MaxBusClkFrec: csd_tab[3],
                                /* Byte 4, 5 */
                                CardComdClasses: (u16::from(csd_tab[4]) << 4) | ((u16::from(csd_tab[5]) & 0xF0) >> 4),
                                /* Byte 5 */
                                RdBlockLen: csd_tab[5] & 0x0F,
                                /* Byte 6 */
                                PartBlockRead: (csd_tab[6] & 0x80) >> 7,
                                WrBlockMisalign: (csd_tab[6] & 0x40) >> 6,
                                RdBlockMisalign: (csd_tab[6] & 0x20) >> 5,
                                DSRImpl: (csd_tab[6] & 0x10) >> 4,
                                Reserved2: 0,
                                // DeviceSize: (csd_tab[6] & 0x03) << 10,
                                /* Byte 7, 8, 9 */
                                DeviceSize: ((u32::from(csd_tab[7]) & 0x3F) << 16)
                                        | (u32::from(csd_tab[8]) << 8)
                                        | u32::from(csd_tab[9]),
                                DeviceSizeMult: 0,
                                /* Byte 10 */
                                EraseGrSize: (csd_tab[10] & 0x40) >> 6,
                                /* Byte 10, 11 */
                                EraseGrMul: ((csd_tab[10] & 0x3F) << 1) | ((csd_tab[11] & 0x80) >> 7),
                                /* Byte 11 */
                                WrProtectGrSize: (csd_tab[11] & 0x7F),
                                /* Byte 12 */
                                WrProtectGrEnable: (csd_tab[12] & 0x80) >> 7,
                                ManDeflECC: (csd_tab[12] & 0x60) >> 5,
                                WrSpeedFact: (csd_tab[12] & 0x1C) >> 2,
                                /* Byte 12,13 */
                                MaxWrBlockLen: ((csd_tab[12] & 0x03) << 2) | ((csd_tab[13] & 0xC0) >> 6),
                                /* Byte 13 */
                                WriteBlockPaPartial: (csd_tab[13] & 0x20) >> 5,
                                Reserved3: 0,
                                ContentProtectAppli: (csd_tab[13] & 0x01),
                                /* Byte 14 */
                                FileFormatGroup: (csd_tab[14] & 0x80) >> 7,
                                CopyFlag: (csd_tab[14] & 0x40) >> 6,
                                PermWrProtect: (csd_tab[14] & 0x20) >> 5,
                                TempWrProtect: (csd_tab[14] & 0x10) >> 4,
                                FileFormat: (csd_tab[14] & 0x0C) >> 2,
                                ECC: (csd_tab[14] & 0x03),
                                /* Byte 15 */
                                CSD_CRC: (csd_tab[15] & 0xFE) >> 1,
                                Reserved4: 1,
                                /* Return the reponse */
                        });
                }
        }

        /// get SD Card CID register
        fn get_cidregister(&self) -> Result<SDCardCID, ()> {
                let mut cid_tab = [0u8; 18];
                /* Send CMD10 (CID register) */
                self.send_cmd(CMD::CMD10, 0, 0);
                /* Wait for response in the R1 format (0x00 is no errors) */
                if self.get_response() != 0x00 {
                        self.end_cmd();
                        return Err(());
                }
                if self.get_response() != SD_START_DATA_SINGLE_BLOCK_READ {
                        self.end_cmd();
                        return Err(());
                }
                /* Store CID register value on cid_tab */
                /* Get CRC bytes (not really needed by us, but required by SD) */
                self.read_data(&mut cid_tab);
                self.end_cmd();
                return Ok(SDCardCID {
                        /* Byte 0 */
                        ManufacturerID: cid_tab[0],
                        /* Byte 1, 2 */
                        OEM_AppliID: (u16::from(cid_tab[1]) << 8) | u16::from(cid_tab[2]),
                        /* Byte 3, 4, 5, 6 */
                        ProdName1: (u32::from(cid_tab[3]) << 24)
                                | (u32::from(cid_tab[4]) << 16)
                                | (u32::from(cid_tab[5]) << 8)
                                | u32::from(cid_tab[6]),
                        /* Byte 7 */
                        ProdName2: cid_tab[7],
                        /* Byte 8 */
                        ProdRev: cid_tab[8],
                        /* Byte 9, 10, 11, 12 */
                        ProdSN: (u32::from(cid_tab[9]) << 24)
                                | (u32::from(cid_tab[10]) << 16)
                                | (u32::from(cid_tab[11]) << 8)
                                | u32::from(cid_tab[12]),
                        /* Byte 13, 14 */
                        Reserved1: (cid_tab[13] & 0xF0) >> 4,
                        ManufactDate: ((u16::from(cid_tab[13]) & 0x0F) << 8) | u16::from(cid_tab[14]),
                        /* Byte 15 */
                        CID_CRC: (cid_tab[15] & 0xFE) >> 1,
                        Reserved2: 1,
                });
        }

        /// Get SD Card info
        fn get_cardinfo(&self) -> Result<SDCardInfo, ()> {
                let mut info = SDCardInfo {
                    SD_csd: self.get_csdregister()?,
                    SD_cid: self.get_cidregister()?,
                    CardCapacity: 0,
                    CardBlockSize: 0,
                    CardBlockCnt: 0,
                };

                if info.SD_csd.CSDStruct == 0 {
                        info.CardBlockSize = (1 as u64) << info.SD_csd.RdBlockLen;
                        let mult = (1 as u64) << (info.SD_csd.DeviceSizeMult + 2);
                        info.CardBlockCnt = (info.SD_csd.DeviceSize + 1) as u64 * mult;
                        info.CardCapacity = info.CardBlockCnt * info.CardBlockSize;
                } else {
                        info.CardBlockSize = 1 << u64::from(info.SD_csd.RdBlockLen);
                        info.CardBlockCnt = (u64::from(info.SD_csd.DeviceSize) + 1) * 1024;
                        info.CardCapacity = info.CardBlockCnt * 512;
                }
                Ok(info)
        }

        /// wait and get data from SD Card
        fn get_dataresponse(&self) -> u8 {
                let response = &mut [0u8];
                /* Read resonse */
                self.read_data(response);
                /* Mask unused bits */
                response[0] &= 0x1F;
                if response[0] != 0x05 {
                        return 0xFF;
                }
                /* Wait null data */
                self.read_data(response);
                while response[0] == 0 {
                        self.read_data(response);
                }
                /* Return response */
                return 0;
        }
        

        /// Initialize an SD Card
        fn init(&mut self) -> Result<SDCardInfo, InitError> {
                self.lowlevel_init();
                self.CS_HIGH();
                self.write_data(&[0xff; 10]);
                self.send_cmd(CMD::CMD0, 0, 0x95);
                let result = self.get_response();
                self.end_cmd();
                if result != 0x01 {
                        return Err(InitError::CMDFailed(CMD::CMD0, result));
                }

                self.send_cmd(CMD::CMD8, 0x01AA, 0x87);
                let result = self.get_response();
                let mut buf = [0u8;4];
                self.read_data(&mut buf);
                self.end_cmd();
                if result != 0x01 {
                        return Err(InitError::CMDFailed(CMD::CMD8, result));
                }
                let mut loop_cnt = 255;
                while loop_cnt != 0 {
                        self.send_cmd(CMD::CMD55, 0, 0);
                        let result = self.get_response();
                        self.end_cmd();
                        if result != 0x01 {
                                return Err(InitError::CMDFailed(CMD::CMD55,result));
                        }

                        self.send_cmd(CMD::ACMD41, 0x40000000, 0);
                        let result = self.get_response();
                        self.end_cmd();
                        if result == 0x00 {
                                break;
                        }
                        loop_cnt -= 1;
                }
                if loop_cnt == 0 {
                        return Err(InitError::CMDFailed(CMD::ACMD41, result));
                }
                loop_cnt = 255;
                let mut frame = [0u8; 4];
                while loop_cnt != 0 {
                        self.send_cmd(CMD::CMD58, 0, 1);
                        let result = self.get_response();
                        self.read_data(&mut frame);
                        self.end_cmd();
                        if result == 0 {
                                break;
                        }
                        loop_cnt -= 1;
                }
                if loop_cnt == 0 {
                        return Err(InitError::CMDFailed(CMD::CMD58, result));
                }
                if (frame[0] & 0x40) == 0 {
                        self.byte_addr = true;
                }
                self.HIGH_SPEED_ENABLE();
                self.get_cardinfo().map_err(|_| InitError::CannotGetCardInfo)
        }

        /// read a sector in the SD Card
        pub fn read_sector(&self, data_buf: &mut [u8], sector: u32) -> Result<(), ()> {
                if data_buf.len() < SEC_LEN || (data_buf.len() % SEC_LEN) != 0 {
                        return Err(());
                }
                let sector = if self.byte_addr {
                        sector * 512
                } else {
                        sector
                };
                /* Send CMD17 to read one block, or CMD18 for multiple */
                let flag = if data_buf.len() == SEC_LEN {
                        self.send_cmd(CMD::CMD17, sector, 0);
                        false
                } else {
                        self.send_cmd(CMD::CMD18, sector, 0);
                        true
                };
                /* Check if the SD acknowledged the read block command: R1 response (0x00: no errors) */
                if self.get_response() != 0x00 {
                        self.end_cmd();
                        return Err(());
                }
                let mut error = false;
                let mut tmp_chunk= [0u8; SEC_LEN];
                for chunk in data_buf.chunks_mut(SEC_LEN) {
                        if self.get_response() != SD_START_DATA_SINGLE_BLOCK_READ {
                                error = true;
                                break;
                        }
                        /* Read the SD block data : read NumByteToRead data */
                        //self.read_data_dma(&mut dma_chunk);
                        self.read_data(&mut tmp_chunk);
                        /* Place the data received as u32 units from DMA into the u8 target buffer */
                        for (a, b) in chunk.iter_mut().zip(/*dma_chunk*/tmp_chunk.iter()) {
                                //*a = (b & 0xff) as u8;
                                *a = *b;
                        }
                        /* Get CRC bytes (not really needed by us, but required by SD) */
                        let mut frame = [0u8; 2];
                        self.read_data(&mut frame);
                        // for i in 0..32 {
                        //         for j in 0..16 {
                        //                 print!("{:02X} ", tmp_chunk[i * 16 + j]);
                        //         }
                        //         println!();
                        // }
                        // println!();
                }
                self.end_cmd();
                if flag {
                        self.send_cmd(CMD::CMD12, 0, 0);
                        self.get_response();
                        self.end_cmd();
                        self.end_cmd();
                }
                /* It is an error if not everything requested was read */
                if error {
                        Err(())
                } else {
                        Ok(())
                }
        }

        /// write data to a sector on SD Card
        pub fn write_sector(&self, data_buf: &[u8], sector: u32) -> Result<(), ()> {
                if data_buf.len() < SEC_LEN || (data_buf.len() % SEC_LEN) != 0 {
                        return Err(());
                }
                let sector = if self.byte_addr {
                        sector * 512
                } else {
                        sector
                };
                let mut frame = [0xff, 0x00];
                if data_buf.len() == SEC_LEN {
                        frame[1] = SD_START_DATA_SINGLE_BLOCK_WRITE;
                        self.send_cmd(CMD::CMD24, sector, 0);
                } else {
                        frame[1] = SD_START_DATA_MULTIPLE_BLOCK_WRITE;
                        self.send_cmd(
                                CMD::ACMD23,
                                (data_buf.len() / SEC_LEN).try_into().unwrap(),
                                0,
                        );
                        self.get_response();
                        self.end_cmd();
                        self.send_cmd(CMD::CMD25, sector, 0);
                }
                /* Check if the SD acknowledged the write block command: R1 response (0x00: no errors) */
                if self.get_response() != 0x00 {
                        self.end_cmd();
                        return Err(());
                }
                //let mut dma_chunk = [0u32; SEC_LEN];
                let mut tmp_chunk = [0u8; SEC_LEN];
                for chunk in data_buf.chunks(SEC_LEN) {
                        /* Send the data token to signify the start of the data */
                        self.write_data(&frame);
                        /* Write the block data to SD : write count data by block */
                        for (a, &b) in /*dma_chunk*/tmp_chunk.iter_mut().zip(chunk.iter()) {
                                //*a = b.into();
                                *a = b;
                        }
                        //self.write_data_dma(&mut dma_chunk);
                        self.write_data(&mut tmp_chunk);
                        /* Put dummy CRC bytes */
                        self.write_data(&[0xff, 0xff]);
                        /* Read data response */
                        if self.get_dataresponse() != 0x00 {
                                self.end_cmd();
                                return Err(());
                        }
                }
                self.end_cmd();
                self.end_cmd();
                Ok(())
        }
}

/// SD Card SPI interface CS pin gpio
const SD_CS_GPIONUM: u8 = 7;

/// SD Card SPI interface CS pin
const SD_CS: u32 = 3;

lazy_static! {
        /// Lazy initialized k210 peripherals
        static ref PERIPHERALS: Mutex<Peripherals> = Mutex::new(Peripherals::take().unwrap());
        // pub static ref sdcard_inst: SDCard0WithLock = SDCard0WithLock::new();
}

/// initialize io
fn io_init() {
        fpioa::set_function(io::SPI0_SCLK, fpioa::function::SPI0_SCLK);
        fpioa::set_function(io::SPI0_MOSI, fpioa::function::SPI0_D0);
        fpioa::set_function(io::SPI0_MISO, fpioa::function::SPI0_D1);
        fpioa::set_function(io::SPI0_CS0, fpioa::function::gpiohs(SD_CS_GPIONUM));
        fpioa::set_io_pull(io::SPI0_CS0, fpioa::pull::DOWN); // GPIO output=pull down
}

/// initialized SD Card
fn init_sdcard() -> SDCard0 {
        usleep(100000);
        let peripherals = unsafe { Peripherals::steal() };
        sysctl::pll_set_freq(sysctl::pll::PLL0, 800_000_000).unwrap();
        sysctl::pll_set_freq(sysctl::pll::PLL1, 300_000_000).unwrap();
        sysctl::pll_set_freq(sysctl::pll::PLL2, 45_158_400).unwrap();
        let clocks = k210_hal::clock::Clocks::new();
        peripherals.UARTHS.configure(115_200.bps(), &clocks);
        io_init();

        let spi = peripherals.SPI0.constrain();

        
        let mut sd = SDCard0{
                        spi: spi, 
                        spi_cs: SD_CS, 
                        cs_gpionum: SD_CS_GPIONUM,
                        byte_addr: false,
                        info: None,
                };
        let info = sd.init().unwrap();
        info!("SDcard (size {}MiB) inited", info.CardCapacity / 1024 / 1024 );
        // println!("SDcard size: {}", info.CardCapacity);
        sd.info = Some(info);
        let sd = sd;
        sd
}

/// SD Card with lock to prevent data racing
pub struct SDCard0WithLock(Mutex<SDCard0>);

impl SDCard0WithLock {
        /// constructor
        pub fn new() -> Self {
                Self(Mutex::new(init_sdcard()))
        }
}

const ZEROS: [u8;512] = [0u8; 512];
impl BlockDevice for SDCard0WithLock {
        fn read_block(&self, block_id: usize, buf: &mut [u8]) {
                self.0.lock().read_sector(buf,block_id as u32).unwrap();
        }
        fn write_block(&self, block_id: usize, buf: &[u8]) {
                self.0.lock().write_sector(buf,block_id as u32).unwrap();
        }
        fn clear_block(&self, block_id: usize) {
                self.0.lock().write_sector(&ZEROS, block_id as u32).unwrap();
        }
        fn block_cnt(&self) -> u64{
                let info = self.0.lock().info.unwrap();
                info.CardBlockCnt * (info.CardBlockSize >> 9)
        }
}
