# # switch stack, thus switch op flow

# .section .text
# .globl __switch
# __switch:
#         sd ra, 0(a0)
#         sd sp, 8(a0)
#         sd s0, 16(a0)
#         sd s1, 24(a0)
#         sd s2, 32(a0)
#         sd s3, 40(a0)
#         sd s4, 48(a0)
#         sd s5, 56(a0)
#         sd s6, 64(a0)
#         sd s7, 72(a0)
#         sd s8, 80(a0)
#         sd s9, 88(a0)
#         sd s10, 96(a0)
#         sd s11, 104(a0)

#         ld ra, 0(a1)
#         ld sp, 8(a1)
#         ld s0, 16(a1)
#         ld s1, 24(a1)
#         ld s2, 32(a1)
#         ld s3, 40(a1)
#         ld s4, 48(a1)
#         ld s5, 56(a1)
#         ld s6, 64(a1)
#         ld s7, 72(a1)
#         ld s8, 80(a1)
#         ld s9, 88(a1)
#         ld s10, 96(a1)
#         ld s11, 104(a1)
        
#         ret


.altmacro
.macro SAVE_SN n
    sd s\n, (\n+1)*8(sp)
.endm
.macro LOAD_SN n
    ld s\n, (\n+1)*8(sp)
.endm
    .section .text
    .globl __switch
__switch:
    # __switch(
    #     current_process_cx_ptr: &*const ProcessContext,
    #     next_process_cx_ptr: &*const ProcessContext
    # )
    # push ProcessContext to current sp and save its address to where a0 points to
    addi sp, sp, -13*8
    sd sp, 0(a0)
    # fill ProcessContext with ra & s0-s11
    sd ra, 0(sp)
    .set n, 0
    .rept 12
        SAVE_SN %n
        .set n, n + 1
    .endr
    # ready for loading ProcessContext a1 points to
    ld sp, 0(a1)
    # load registers in the ProcessContext
    ld ra, 0(sp)
    .set n, 0
    .rept 12
        LOAD_SN %n
        .set n, n + 1
    .endr
    # pop ProcessContext
    addi sp, sp, 13*8
    ret

