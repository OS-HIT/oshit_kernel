macro_rules! EnumWithTryFrom {
    ($(#[$meta:meta])* $vis:vis enum $name:ident {
        $($(#[$vmeta:meta])* $vname:ident $(= $val:expr)?,)*
    }) => {
        $(#[$meta])*
        $vis enum $name {
            $($(#[$vmeta])* $vname $(= $val)?,)*
        }

        impl core::convert::TryFrom<u64> for $name {
            type Error = ();

            fn try_from(v: u64) -> Result<Self, Self::Error> {
                match v {
                    $(x if x == $name::$vname as u64 => Ok($name::$vname),)*
                    _ => Err(()),
                }
            }
        }
    }
}

EnumWithTryFrom!{
    #[repr(u64)]
    #[derive(Clone, Copy, Debug)]
    pub enum ErrNo {
		OperationNotPermitted                       = 1  ,
		NoSuchFileOrDirectory                       = 2  ,
		NoSuchProcess                               = 3  ,
		InterruptedSystemCall                       = 4  ,
		IOError                                     = 5  ,
		NoSuchDeviceOrAddress                       = 6  ,
		ArgumentListTooLong                         = 7  ,
		ExecFormatError                             = 8  ,
		BadFileDescriptor                               = 9  ,
		NoChildProcesses                            = 10 ,
		TryAgain                                    = 11 ,
		OutOfMemory                                 = 12 ,
		PermissionDenied                            = 13 ,
		BadAddress                                  = 14 ,
		BlockDeviceRequired                         = 15 ,
		DeviceOrResourceBusy                        = 16 ,
		FileExists                                  = 17 ,
		CrossdeviceLink                             = 18 ,
		NotSuchDevice                                = 19 ,
		NotADirectory                               = 20 ,
		IsADirectory                                = 21 ,
		InvalidArgument                             = 22 ,
		FileTableOverflow                           = 23 ,
		TooManyOpenFiles                            = 24 ,
		NotATypewriter                              = 25 ,
		TextFileBusy                                = 26 ,
		FileTooLarge                                = 27 ,
		NoSpaceLeftOnDevice                         = 28 ,
		IllegalSeek                                 = 29 ,
		ReadonlyFileSystem                          = 30 ,
		TooManyLinks                                = 31 ,
		BrokenPipe                                  = 32 ,
		MathArgumentOutOfDomainOfFunc               = 33 ,
		MathResultNotRepresentable                  = 34 ,
		ResourceDeadlockWouldOccur                  = 35 ,
		FileNameTooLong                             = 36 ,
		NoRecordLocksAvailable                      = 37 ,
		FunctionNotImplemented                      = 38 ,
		DirectoryNotEmpty                           = 39 ,
		TooManySymbolicLinksEncountered             = 40 ,
		NoMessageOfDesiredType                      = 42 ,
		IdentifierRemoved                           = 43 ,
		ChannelNumberOutOfRange                     = 44 ,
		Level2NotSynchronized                       = 45 ,
		Level3Halted                                = 46 ,
		Level3Reset                                 = 47 ,
		LinkNumberOutOfRange                        = 48 ,
		ProtocolDriverNotAttached                   = 49 ,
		NoCSIStructureAvailable                     = 50 ,
		Level2Halted                                = 51 ,
		InvalidExchange                             = 52 ,
		InvalidRequestDescriptor                    = 53 ,
		ExchangeFull                                = 54 ,
		NoAnode                                     = 55 ,
		InvalidRequestCode                          = 56 ,
		InvalidSlot                                 = 57 ,
		BadFontFileFormat                           = 59 ,
		DeviceNotAStream                            = 60 ,
		NoDataAvailable                             = 61 ,
		TimerExpired                                = 62 ,
		OutOfStreamsResources                       = 63 ,
		MachineIsNotOnTheNetwork                    = 64 ,
		PackageNotInstalled                         = 65 ,
		ObjectIsRemote                              = 66 ,
		LinkHasBeenSevered                          = 67 ,
		AdvertiseError                              = 68 ,
		SrmountError                                = 69 ,
		CommunicationErrorOnSend                    = 70 ,
		ProtocolError                               = 71 ,
		MultihopAttempted                           = 72 ,
		RFSSpecificError                            = 73 ,
		NotADataMessage                             = 74 ,
		ValueTooLargeForDefinedDataType             = 75 ,
		NameNotUniqueOnNetwork                      = 76 ,
		FileDescriptorInBadState                    = 77 ,
		RemoteAddressChanged                        = 78 ,
		CanNotAccessANeededSharedLibrary            = 79 ,
		AccessingACorruptedSharedLibrary            = 80 ,
		LibSectionCorrupted                         = 81 ,
		AttemptingToLinkInTooManySharedLibraries    = 82 ,
		CannotExecASharedLibraryDirectly            = 83 ,
		IllegalByteSequence                         = 84 ,
		InterruptedSystemCallShouldBeRestarted      = 85 ,
		StreamsPipeError                            = 86 ,
		TooManyUsers                                = 87 ,
		SocketOperationOnNonsocket                  = 88 ,
		DestinationAddressRequired                  = 89 ,
		MessageTooLong                              = 90 ,
		ProtocolWrongTypeForSocket                  = 91 ,
		ProtocolNotAvailable                        = 92 ,
		ProtocolNotSupported                        = 93 ,
		SocketTypeNotSupported                      = 94 ,
		OperationNotSupportedOnTransportEndpoint    = 95 ,
		ProtocolFamilyNotSupported                  = 96 ,
		AddressFamilyNotSupportedByProtocol         = 97 ,
		AddressAlreadyInUse                         = 98 ,
		CannotAssignRequestedAddress                = 99 ,
		NetworkIsDown                               = 100,
		NetworkIsUnreachable                        = 101,
		NetworkDroppedConnectionBecauseOfReset      = 102,
		SoftwareCausedConnectionAbort               = 103,
		ConnectionResetByPeer                       = 104,
		NoBufferSpaceAvailable                      = 105,
		TransportEndpointIsAlreadyConnected         = 106,
		TransportEndpointIsNotConnected             = 107,
		CannotSendAfterTransportEndpointShutdown    = 108,
		TooManyReferencesCannotSplice               = 109,
		ConnectionTimedOut                          = 110,
		ConnectionRefused                           = 111,
		HostIsDown                                  = 112,
		NoRouteToHost                               = 113,
		OperationAlreadyInProgress                  = 114,
		OperationNowInProgress                      = 115,
		StaleNFSFileHandle                          = 116,
		StructureNeedsCleaning                      = 117,
		NotAXENIXNamedTypeFile                      = 118,
		NoXENIXSemaphoresAvailable                  = 119,
		IsANamedTypeFile                            = 120,
		RemoteIOError                               = 121,
		QuotaExceeded                               = 122,
		NoMediumFound                               = 123,
		WrongMediumType                             = 124,
		OperationCanceled                           = 125,
		RequiredKeyNotAvailable                     = 126,
		KeyHasExpired                               = 127,
		KeyHasBeenRevoked                           = 128,
		KeyWasRejectedByService                     = 129,
		OwnerDied                                   = 130,
		StateNotRecoverable                         = 131,

		Fat32FakeInode					= 1000,
		Fat32InvalidOffset				= 1001,
		Fat32EntryShortRead				= 1002,
		Fat32NoMoreEntry				= 1003,
	}
}

impl core::fmt::Display for ErrNo {
	fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
		core::fmt::Debug::fmt(self, f)
	}
}