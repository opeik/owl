use std::ffi::c_int;

use cec_sys::*;
use enum_repr::EnumRepr;

use crate::TryFromLogicalAddressesError;

#[EnumRepr(type = "cec_abort_reason")]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum AbortReason {
    UnrecognizedOpcode = cec_abort_reason::UNRECOGNIZED_OPCODE,
    NotInCorrectModeToRespond = cec_abort_reason::NOT_IN_CORRECT_MODE_TO_RESPOND,
    CannotProvideSource = cec_abort_reason::CANNOT_PROVIDE_SOURCE,
    InvalidOperand = cec_abort_reason::INVALID_OPERAND,
    Refused = cec_abort_reason::REFUSED,
}

#[EnumRepr(type = "cec_analogue_broadcast_type")]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum AnalogueBroadcastType {
    Cable = cec_analogue_broadcast_type::CABLE,
    Satellite = cec_analogue_broadcast_type::SATELLITE,
    Terrestial = cec_analogue_broadcast_type::TERRESTIAL,
}

#[EnumRepr(type = "cec_audio_rate")]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum AudioRate {
    RateControlOff = cec_audio_rate::RATE_CONTROL_OFF,
    StandardRate100 = cec_audio_rate::STANDARD_RATE_100,
    FastRateMax101 = cec_audio_rate::FAST_RATE_MAX_101,
    SlowRateMin99 = cec_audio_rate::SLOW_RATE_MIN_99,
    StandardRate1000 = cec_audio_rate::STANDARD_RATE_100_0,
    FastRateMax1001 = cec_audio_rate::FAST_RATE_MAX_100_1,
    SlowRateMin999 = cec_audio_rate::SLOW_RATE_MIN_99_9,
}

#[EnumRepr(type = "cec_audio_status")]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum AudioStatus {
    MuteStatusMask = cec_audio_status::MUTE_STATUS_MASK,
    VolumeStatusMask = cec_audio_status::VOLUME_STATUS_MASK,
    VolumeMin = cec_audio_status::VOLUME_MIN,
    VolumeMax = cec_audio_status::VOLUME_MAX,
}

#[EnumRepr(type = "cec_version")]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Version {
    VersionUnknown = cec_version::UNKNOWN,
    Version12 = cec_version::_1_2,
    Version12a = cec_version::_1_2A,
    Version13 = cec_version::_1_3,
    Version13a = cec_version::_1_3A,
    Version14 = cec_version::_1_4,
    Version20 = cec_version::_2_0,
}

#[EnumRepr(type = "cec_channel_identifier")]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum ChannelIdentifier {
    CecChannelNumberFormatMask = cec_channel_identifier::CEC_CHANNEL_NUMBER_FORMAT_MASK,
    Cec1PartChannelNumber = cec_channel_identifier::CEC_1_PART_CHANNEL_NUMBER,
    Cec2PartChannelNumber = cec_channel_identifier::CEC_2_PART_CHANNEL_NUMBER,
    CecMajorChannelNumberMask = cec_channel_identifier::CEC_MAJOR_CHANNEL_NUMBER_MASK,
    CecMinorChannelNumberMask = cec_channel_identifier::CEC_MINOR_CHANNEL_NUMBER_MASK,
}

#[EnumRepr(type = "cec_deck_control_mode")]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum DeckControlMode {
    SkipForwardWind = cec_deck_control_mode::SKIP_FORWARD_WIND,
    SkipReverseRewind = cec_deck_control_mode::SKIP_REVERSE_REWIND,
    Stop = cec_deck_control_mode::STOP,
    Eject = cec_deck_control_mode::EJECT,
}

#[EnumRepr(type = "cec_deck_info")]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum DeckInfo {
    Play = cec_deck_info::PLAY,
    Record = cec_deck_info::RECORD,
    PlayReverse = cec_deck_info::PLAY_REVERSE,
    Still = cec_deck_info::STILL,
    Slow = cec_deck_info::SLOW,
    SlowReverse = cec_deck_info::SLOW_REVERSE,
    FastForward = cec_deck_info::FAST_FORWARD,
    FastReverse = cec_deck_info::FAST_REVERSE,
    NoMedia = cec_deck_info::NO_MEDIA,
    Stop = cec_deck_info::STOP,
    SkipForwardWind = cec_deck_info::SKIP_FORWARD_WIND,
    SkipReverseRewind = cec_deck_info::SKIP_REVERSE_REWIND,
    IndexSearchForward = cec_deck_info::INDEX_SEARCH_FORWARD,
    IndexSearchReverse = cec_deck_info::INDEX_SEARCH_REVERSE,
    OtherStatus = cec_deck_info::OTHER_STATUS,
    OtherStatusLg = cec_deck_info::OTHER_STATUS_LG,
}

#[EnumRepr(type = "cec_device_type")]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum DeviceKind {
    Tv = cec_device_type::TV,
    RecordingDevice = cec_device_type::RECORDING_DEVICE,
    Reserved = cec_device_type::RESERVED,
    Tuner = cec_device_type::TUNER,
    PlaybackDevice = cec_device_type::PLAYBACK_DEVICE,
    AudioSystem = cec_device_type::AUDIO_SYSTEM,
}

#[EnumRepr(type = "cec_display_control")]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum DisplayControl {
    DisplayForDefaultTime = cec_display_control::DISPLAY_FOR_DEFAULT_TIME,
    DisplayUntilCleared = cec_display_control::DISPLAY_UNTIL_CLEARED,
    ClearPreviousMessage = cec_display_control::CLEAR_PREVIOUS_MESSAGE,
    ReservedForFutureUse = cec_display_control::RESERVED_FOR_FUTURE_USE,
}

#[EnumRepr(type = "cec_external_source_specifier")]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum ExternalSourceSpecifier {
    Plug = cec_external_source_specifier::EXTERNAL_PLUG,
    PhysicalAddress = cec_external_source_specifier::EXTERNAL_PHYSICAL_ADDRESS,
}

#[EnumRepr(type = "cec_menu_request_type")]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum MenuRequestType {
    Activate = cec_menu_request_type::ACTIVATE,
    Deactivate = cec_menu_request_type::DEACTIVATE,
    Query = cec_menu_request_type::QUERY,
}

#[EnumRepr(type = "cec_menu_state")]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum MenuState {
    Activated = cec_menu_state::ACTIVATED,
    Deactivated = cec_menu_state::DEACTIVATED,
}

#[EnumRepr(type = "cec_play_mode")]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum PlayMode {
    PlayForward = cec_play_mode::PLAY_FORWARD,
    PlayReverse = cec_play_mode::PLAY_REVERSE,
    PlayStill = cec_play_mode::PLAY_STILL,
    FastForwardMinSpeed = cec_play_mode::FAST_FORWARD_MIN_SPEED,
    FastForwardMediumSpeed = cec_play_mode::FAST_FORWARD_MEDIUM_SPEED,
    FastForwardMaxSpeed = cec_play_mode::FAST_FORWARD_MAX_SPEED,
    FastReverseMinSpeed = cec_play_mode::FAST_REVERSE_MIN_SPEED,
    FastReverseMediumSpeed = cec_play_mode::FAST_REVERSE_MEDIUM_SPEED,
    FastReverseMaxSpeed = cec_play_mode::FAST_REVERSE_MAX_SPEED,
    SlowForwardMinSpeed = cec_play_mode::SLOW_FORWARD_MIN_SPEED,
    SlowForwardMediumSpeed = cec_play_mode::SLOW_FORWARD_MEDIUM_SPEED,
    SlowForwardMaxSpeed = cec_play_mode::SLOW_FORWARD_MAX_SPEED,
    SlowReverseMinSpeed = cec_play_mode::SLOW_REVERSE_MIN_SPEED,
    SlowReverseMediumSpeed = cec_play_mode::SLOW_REVERSE_MEDIUM_SPEED,
    SlowReverseMaxSpeed = cec_play_mode::SLOW_REVERSE_MAX_SPEED,
}

#[EnumRepr(type = "cec_power_status")]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum PowerStatus {
    On = cec_power_status::ON,
    Standby = cec_power_status::STANDBY,
    InTransitionStandbyToOn = cec_power_status::IN_TRANSITION_STANDBY_TO_ON,
    InTransitionOnToStandby = cec_power_status::IN_TRANSITION_ON_TO_STANDBY,
    Unknown = cec_power_status::UNKNOWN,
}

#[EnumRepr(type = "cec_record_source_type")]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum RecordSourceType {
    OwnSource = cec_record_source_type::OWN_SOURCE,
    DigitalService = cec_record_source_type::DIGITAL_SERVICE,
    AnalogueService = cec_record_source_type::ANALOGUE_SERVICE,
    ExternalPlus = cec_record_source_type::EXTERNAL_PLUS,
    ExternalPhysicalAddress = cec_record_source_type::EXTERNAL_PHYSICAL_ADDRESS,
}

#[EnumRepr(type = "cec_record_status_info")]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum RecordStatusInfo {
    RecordingCurrentlySelectedSource = cec_record_status_info::RECORDING_CURRENTLY_SELECTED_SOURCE,
    RecordingDigitalService = cec_record_status_info::RECORDING_DIGITAL_SERVICE,
    RecordingAnalogueService = cec_record_status_info::RECORDING_ANALOGUE_SERVICE,
    RecordingExternalInput = cec_record_status_info::RECORDING_EXTERNAL_INPUT,
    NoRecordingUnableToRecordDigitalService =
        cec_record_status_info::NO_RECORDING_UNABLE_TO_RECORD_DIGITAL_SERVICE,
    NoRecordingUnableToRecordAnalogueService =
        cec_record_status_info::NO_RECORDING_UNABLE_TO_RECORD_ANALOGUE_SERVICE,
    NoRecordingUnableToSelectRequiredService =
        cec_record_status_info::NO_RECORDING_UNABLE_TO_SELECT_REQUIRED_SERVICE,
    NoRecordingInvalidExternalPlugNumber =
        cec_record_status_info::NO_RECORDING_INVALID_EXTERNAL_PLUG_NUMBER,
    NoRecordingInvalidExternalAddress =
        cec_record_status_info::NO_RECORDING_INVALID_EXTERNAL_ADDRESS,
    NoRecordingCaSystemNotSupported = cec_record_status_info::NO_RECORDING_CA_SYSTEM_NOT_SUPPORTED,
    NoRecordingNoOrInsufficientEntitlements =
        cec_record_status_info::NO_RECORDING_NO_OR_INSUFFICIENT_ENTITLEMENTS,
    NoRecordingNotAllowedToCopySource =
        cec_record_status_info::NO_RECORDING_NOT_ALLOWED_TO_COPY_SOURCE,
    NoRecordingNoFurtherCopiesAllowed =
        cec_record_status_info::NO_RECORDING_NO_FURTHER_COPIES_ALLOWED,
    NoRecordingNoMedia = cec_record_status_info::NO_RECORDING_NO_MEDIA,
    NoRecordingPlaying = cec_record_status_info::NO_RECORDING_PLAYING,
    NoRecordingAlreadyRecording = cec_record_status_info::NO_RECORDING_ALREADY_RECORDING,
    NoRecordingMediaProtected = cec_record_status_info::NO_RECORDING_MEDIA_PROTECTED,
    NoRecordingNoSourceSignal = cec_record_status_info::NO_RECORDING_NO_SOURCE_SIGNAL,
    NoRecordingMediaProblem = cec_record_status_info::NO_RECORDING_MEDIA_PROBLEM,
    NoRecordingNotEnoughSpaceAvailable =
        cec_record_status_info::NO_RECORDING_NOT_ENOUGH_SPACE_AVAILABLE,
    NoRecordingParentalLockOn = cec_record_status_info::NO_RECORDING_PARENTAL_LOCK_ON,
    RecordingTerminatedNormally = cec_record_status_info::RECORDING_TERMINATED_NORMALLY,
    RecordingHasAlreadyTerminated = cec_record_status_info::RECORDING_HAS_ALREADY_TERMINATED,
    NoRecordingOtherReason = cec_record_status_info::NO_RECORDING_OTHER_REASON,
}

#[EnumRepr(type = "cec_recording_sequence")]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum RecordingSequence {
    Sunday = cec_recording_sequence::SUNDAY,
    Monday = cec_recording_sequence::MONDAY,
    Tuesday = cec_recording_sequence::TUESDAY,
    Wednesday = cec_recording_sequence::WEDNESDAY,
    Thursday = cec_recording_sequence::THURSDAY,
    Friday = cec_recording_sequence::FRIDAY,
    Saturday = cec_recording_sequence::SATURDAY,
    OnceOnly = cec_recording_sequence::ONCE_ONLY,
}

#[EnumRepr(type = "cec_status_request")]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum StatusRequest {
    On = cec_status_request::ON,
    Off = cec_status_request::OFF,
    Once = cec_status_request::ONCE,
}

#[EnumRepr(type = "cec_system_audio_status")]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum SystemAudioStatus {
    Off = cec_system_audio_status::OFF,
    On = cec_system_audio_status::ON,
}

#[EnumRepr(type = "cec_timer_cleared_status_data")]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum TimerClearedStatusData {
    NotClearedRecording = cec_timer_cleared_status_data::TIMER_NOT_CLEARED_RECORDING,
    NotClearedNoMatching = cec_timer_cleared_status_data::TIMER_NOT_CLEARED_NO_MATCHING,
    NotClearedNoInf0Available = cec_timer_cleared_status_data::TIMER_NOT_CLEARED_NO_INF0_AVAILABLE,
    Cleared = cec_timer_cleared_status_data::TIMER_CLEARED,
}

#[EnumRepr(type = "cec_timer_overlap_warning")]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum TimerOverlapWarning {
    NoOverlap = cec_timer_overlap_warning::NO_OVERLAP,
    TimerBlocksOverlap = cec_timer_overlap_warning::TIMER_BLOCKS_OVERLAP,
}

#[EnumRepr(type = "cec_media_info")]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum MediaInfo {
    MediaPresentAndNotProtected = cec_media_info::MEDIA_PRESENT_AND_NOT_PROTECTED,
    MediaPresentButProtected = cec_media_info::MEDIA_PRESENT_BUT_PROTECTED,
    MediaNotPresent = cec_media_info::MEDIA_NOT_PRESENT,
    FutureUse = cec_media_info::FUTURE_USE,
}

#[EnumRepr(type = "cec_programmed_indicator")]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum ProgrammedIndicator {
    NotProgrammed = cec_programmed_indicator::NOT_PROGRAMMED,
    Programmed = cec_programmed_indicator::PROGRAMMED,
}

#[EnumRepr(type = "cec_programmed_info")]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum ProgrammedInfo {
    FutureUse = cec_programmed_info::FUTURE_USE,
    EnoughSpaceAvailableForRecording = cec_programmed_info::ENOUGH_SPACE_AVAILABLE_FOR_RECORDING,
    NotEnoughSpaceAvailableForRecording =
        cec_programmed_info::NOT_ENOUGH_SPACE_AVAILABLE_FOR_RECORDING,
    MayNotBeEnoughSpaceAvailable = cec_programmed_info::MAY_NOT_BE_ENOUGH_SPACE_AVAILABLE,
    NoMediaInfoAvailable = cec_programmed_info::NO_MEDIA_INFO_AVAILABLE,
}

#[EnumRepr(type = "cec_not_programmed_error_info")]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum NotProgrammedErrorInfo {
    FutureUse = cec_not_programmed_error_info::FUTURE_USE,
    NoFreeTimerAvailable = cec_not_programmed_error_info::NO_FREE_TIMER_AVAILABLE,
    DateOutOfRange = cec_not_programmed_error_info::DATE_OUT_OF_RANGE,
    RecordingSequenceError = cec_not_programmed_error_info::RECORDING_SEQUENCE_ERROR,
    InvalidExternalPlugNumber = cec_not_programmed_error_info::INVALID_EXTERNAL_PLUG_NUMBER,
    InvalidExternalPhysicalAddress =
        cec_not_programmed_error_info::INVALID_EXTERNAL_PHYSICAL_ADDRESS,
    CaSystemNotSupported = cec_not_programmed_error_info::CA_SYSTEM_NOT_SUPPORTED,
    NoOrInsufficientCaEntitlements =
        cec_not_programmed_error_info::NO_OR_INSUFFICIENT_CA_ENTITLEMENTS,
    DoesNotSupportResolution = cec_not_programmed_error_info::DOES_NOT_SUPPORT_RESOLUTION,
    ParentalLockOn = cec_not_programmed_error_info::PARENTAL_LOCK_ON,
    ClockFailure = cec_not_programmed_error_info::CLOCK_FAILURE,
    ReservedForFutureUseStart = cec_not_programmed_error_info::RESERVED_FOR_FUTURE_USE_START,
    ReservedForFutureUseEnd = cec_not_programmed_error_info::RESERVED_FOR_FUTURE_USE_END,
    DuplicateAlreadyProgrammed = cec_not_programmed_error_info::DUPLICATE_ALREADY_PROGRAMMED,
}

#[EnumRepr(type = "cec_recording_flag")]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum RecordingFlag {
    NotBeingUsedForRecording = cec_recording_flag::NOT_BEING_USED_FOR_RECORDING,
    BeingUsedForRecording = cec_recording_flag::BEING_USED_FOR_RECORDING,
}

#[EnumRepr(type = "cec_tuner_display_info")]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum TunerDisplayInfo {
    DisplayingDigitalTuner = cec_tuner_display_info::DISPLAYING_DIGITAL_TUNER,
    NotDisplayingTuner = cec_tuner_display_info::NOT_DISPLAYING_TUNER,
    DisplayingAnalogueTuner = cec_tuner_display_info::DISPLAYING_ANALOGUE_TUNER,
}

#[EnumRepr(type = "cec_broadcast_system")]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum BroadcastSystem {
    PalBG = cec_broadcast_system::PAL_B_G,
    SecamL1 = cec_broadcast_system::SECAM_L1,
    PalM = cec_broadcast_system::PAL_M,
    NtscM = cec_broadcast_system::NTSC_M,
    PalI = cec_broadcast_system::PAL_I,
    SecamDk = cec_broadcast_system::SECAM_DK,
    SecamBG = cec_broadcast_system::SECAM_B_G,
    SecamL2 = cec_broadcast_system::SECAM_L2,
    PalDk = cec_broadcast_system::PAL_DK,
    OtherSystem = cec_broadcast_system::OTHER_SYSTEM,
}

#[EnumRepr(type = "cec_user_control_code")]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum UserControlCode {
    Select = cec_user_control_code::SELECT,
    Up = cec_user_control_code::UP,
    Down = cec_user_control_code::DOWN,
    Left = cec_user_control_code::LEFT,
    Right = cec_user_control_code::RIGHT,
    RightUp = cec_user_control_code::RIGHT_UP,
    RightDown = cec_user_control_code::RIGHT_DOWN,
    LeftUp = cec_user_control_code::LEFT_UP,
    LeftDown = cec_user_control_code::LEFT_DOWN,
    RootMenu = cec_user_control_code::ROOT_MENU,
    SetupMenu = cec_user_control_code::SETUP_MENU,
    ContentsMenu = cec_user_control_code::CONTENTS_MENU,
    FavoriteMenu = cec_user_control_code::FAVORITE_MENU,
    Exit = cec_user_control_code::EXIT,
    TopMenu = cec_user_control_code::TOP_MENU,
    DvdMenu = cec_user_control_code::DVD_MENU,
    NumberEntryMode = cec_user_control_code::NUMBER_ENTRY_MODE,
    Number11 = cec_user_control_code::NUMBER11,
    Number12 = cec_user_control_code::NUMBER12,
    Number0 = cec_user_control_code::NUMBER0,
    Number1 = cec_user_control_code::NUMBER1,
    Number2 = cec_user_control_code::NUMBER2,
    Number3 = cec_user_control_code::NUMBER3,
    Number4 = cec_user_control_code::NUMBER4,
    Number5 = cec_user_control_code::NUMBER5,
    Number6 = cec_user_control_code::NUMBER6,
    Number7 = cec_user_control_code::NUMBER7,
    Number8 = cec_user_control_code::NUMBER8,
    Number9 = cec_user_control_code::NUMBER9,
    Dot = cec_user_control_code::DOT,
    Enter = cec_user_control_code::ENTER,
    Clear = cec_user_control_code::CLEAR,
    NextFavorite = cec_user_control_code::NEXT_FAVORITE,
    ChannelUp = cec_user_control_code::CHANNEL_UP,
    ChannelDown = cec_user_control_code::CHANNEL_DOWN,
    PreviousChannel = cec_user_control_code::PREVIOUS_CHANNEL,
    SoundSelect = cec_user_control_code::SOUND_SELECT,
    InputSelect = cec_user_control_code::INPUT_SELECT,
    DisplayInformation = cec_user_control_code::DISPLAY_INFORMATION,
    Help = cec_user_control_code::HELP,
    PageUp = cec_user_control_code::PAGE_UP,
    PageDown = cec_user_control_code::PAGE_DOWN,
    Power = cec_user_control_code::POWER,
    VolumeUp = cec_user_control_code::VOLUME_UP,
    VolumeDown = cec_user_control_code::VOLUME_DOWN,
    Mute = cec_user_control_code::MUTE,
    Play = cec_user_control_code::PLAY,
    Stop = cec_user_control_code::STOP,
    Pause = cec_user_control_code::PAUSE,
    Record = cec_user_control_code::RECORD,
    Rewind = cec_user_control_code::REWIND,
    FastForward = cec_user_control_code::FAST_FORWARD,
    Eject = cec_user_control_code::EJECT,
    Forward = cec_user_control_code::FORWARD,
    Backward = cec_user_control_code::BACKWARD,
    StopRecord = cec_user_control_code::STOP_RECORD,
    PauseRecord = cec_user_control_code::PAUSE_RECORD,
    Angle = cec_user_control_code::ANGLE,
    SubPicture = cec_user_control_code::SUB_PICTURE,
    VideoOnDemand = cec_user_control_code::VIDEO_ON_DEMAND,
    ElectronicProgramGuide = cec_user_control_code::ELECTRONIC_PROGRAM_GUIDE,
    TimerProgramming = cec_user_control_code::TIMER_PROGRAMMING,
    InitialConfiguration = cec_user_control_code::INITIAL_CONFIGURATION,
    SelectBroadcastType = cec_user_control_code::SELECT_BROADCAST_TYPE,
    SelectSoundPresentation = cec_user_control_code::SELECT_SOUND_PRESENTATION,
    PlayFunction = cec_user_control_code::PLAY_FUNCTION,
    PausePlayFunction = cec_user_control_code::PAUSE_PLAY_FUNCTION,
    RecordFunction = cec_user_control_code::RECORD_FUNCTION,
    PauseRecordFunction = cec_user_control_code::PAUSE_RECORD_FUNCTION,
    StopFunction = cec_user_control_code::STOP_FUNCTION,
    MuteFunction = cec_user_control_code::MUTE_FUNCTION,
    RestoreVolumeFunction = cec_user_control_code::RESTORE_VOLUME_FUNCTION,
    TuneFunction = cec_user_control_code::TUNE_FUNCTION,
    SelectMediaFunction = cec_user_control_code::SELECT_MEDIA_FUNCTION,
    SelectAvInputFunction = cec_user_control_code::SELECT_AV_INPUT_FUNCTION,
    SelectAudioInputFunction = cec_user_control_code::SELECT_AUDIO_INPUT_FUNCTION,
    PowerToggleFunction = cec_user_control_code::POWER_TOGGLE_FUNCTION,
    PowerOffFunction = cec_user_control_code::POWER_OFF_FUNCTION,
    PowerOnFunction = cec_user_control_code::POWER_ON_FUNCTION,
    F1Blue = cec_user_control_code::F1_BLUE,
    F2Red = cec_user_control_code::F2_RED,
    F3Green = cec_user_control_code::F3_GREEN,
    F4Yellow = cec_user_control_code::F4_YELLOW,
    F5 = cec_user_control_code::F5,
    Data = cec_user_control_code::DATA,
    AnReturn = cec_user_control_code::AN_RETURN,
    AnChannelsList = cec_user_control_code::AN_CHANNELS_LIST,
    Unknown = cec_user_control_code::UNKNOWN,
}

#[EnumRepr(type = "cec_logical_address")]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum LogicalAddress {
    Unknown = cec_logical_address::UNKNOWN,
    Tv = cec_logical_address::TV,
    Recordingdevice1 = cec_logical_address::RECORDINGDEVICE1,
    Recordingdevice2 = cec_logical_address::RECORDINGDEVICE2,
    Tuner1 = cec_logical_address::TUNER1,
    Playbackdevice1 = cec_logical_address::PLAYBACKDEVICE1,
    Audiosystem = cec_logical_address::AUDIOSYSTEM,
    Tuner2 = cec_logical_address::TUNER2,
    Tuner3 = cec_logical_address::TUNER3,
    Playbackdevice2 = cec_logical_address::PLAYBACKDEVICE2,
    Recordingdevice3 = cec_logical_address::RECORDINGDEVICE3,
    Tuner4 = cec_logical_address::TUNER4,
    Playbackdevice3 = cec_logical_address::PLAYBACKDEVICE3,
    Reserved1 = cec_logical_address::RESERVED1,
    Reserved2 = cec_logical_address::RESERVED2,
    Freeuse = cec_logical_address::FREEUSE,
    Unregistered = cec_logical_address::UNREGISTERED,
}

#[EnumRepr(type = "cec_opcode")]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Opcode {
    ActiveSource = cec_opcode::ACTIVE_SOURCE,
    ImageViewOn = cec_opcode::IMAGE_VIEW_ON,
    TextViewOn = cec_opcode::TEXT_VIEW_ON,
    InactiveSource = cec_opcode::INACTIVE_SOURCE,
    RequestActiveSource = cec_opcode::REQUEST_ACTIVE_SOURCE,
    RoutingChange = cec_opcode::ROUTING_CHANGE,
    RoutingInformation = cec_opcode::ROUTING_INFORMATION,
    SetStreamPath = cec_opcode::SET_STREAM_PATH,
    Standby = cec_opcode::STANDBY,
    RecordOff = cec_opcode::RECORD_OFF,
    RecordOn = cec_opcode::RECORD_ON,
    RecordStatus = cec_opcode::RECORD_STATUS,
    RecordTvScreen = cec_opcode::RECORD_TV_SCREEN,
    ClearAnalogueTimer = cec_opcode::CLEAR_ANALOGUE_TIMER,
    ClearDigitalTimer = cec_opcode::CLEAR_DIGITAL_TIMER,
    ClearExternalTimer = cec_opcode::CLEAR_EXTERNAL_TIMER,
    SetAnalogueTimer = cec_opcode::SET_ANALOGUE_TIMER,
    SetDigitalTimer = cec_opcode::SET_DIGITAL_TIMER,
    SetExternalTimer = cec_opcode::SET_EXTERNAL_TIMER,
    SetTimerProgramTitle = cec_opcode::SET_TIMER_PROGRAM_TITLE,
    TimerClearedStatus = cec_opcode::TIMER_CLEARED_STATUS,
    TimerStatus = cec_opcode::TIMER_STATUS,
    CecVersion = cec_opcode::CEC_VERSION,
    GetCecVersion = cec_opcode::GET_CEC_VERSION,
    GivePhysicalAddress = cec_opcode::GIVE_PHYSICAL_ADDRESS,
    GetMenuLanguage = cec_opcode::GET_MENU_LANGUAGE,
    ReportPhysicalAddress = cec_opcode::REPORT_PHYSICAL_ADDRESS,
    SetMenuLanguage = cec_opcode::SET_MENU_LANGUAGE,
    DeckControl = cec_opcode::DECK_CONTROL,
    DeckStatus = cec_opcode::DECK_STATUS,
    GiveDeckStatus = cec_opcode::GIVE_DECK_STATUS,
    Play = cec_opcode::PLAY,
    GiveTunerDeviceStatus = cec_opcode::GIVE_TUNER_DEVICE_STATUS,
    SelectAnalogueService = cec_opcode::SELECT_ANALOGUE_SERVICE,
    SelectDigitalService = cec_opcode::SELECT_DIGITAL_SERVICE,
    TunerDeviceStatus = cec_opcode::TUNER_DEVICE_STATUS,
    TunerStepDecrement = cec_opcode::TUNER_STEP_DECREMENT,
    TunerStepIncrement = cec_opcode::TUNER_STEP_INCREMENT,
    DeviceVendorId = cec_opcode::DEVICE_VENDOR_ID,
    GiveDeviceVendorId = cec_opcode::GIVE_DEVICE_VENDOR_ID,
    VendorCommand = cec_opcode::VENDOR_COMMAND,
    VendorCommandWithId = cec_opcode::VENDOR_COMMAND_WITH_ID,
    VendorRemoteButtonDown = cec_opcode::VENDOR_REMOTE_BUTTON_DOWN,
    VendorRemoteButtonUp = cec_opcode::VENDOR_REMOTE_BUTTON_UP,
    SetOsdString = cec_opcode::SET_OSD_STRING,
    GiveOsdName = cec_opcode::GIVE_OSD_NAME,
    SetOsdName = cec_opcode::SET_OSD_NAME,
    MenuRequest = cec_opcode::MENU_REQUEST,
    MenuStatus = cec_opcode::MENU_STATUS,
    UserControlPressed = cec_opcode::USER_CONTROL_PRESSED,
    UserControlRelease = cec_opcode::USER_CONTROL_RELEASE,
    GiveDevicePowerStatus = cec_opcode::GIVE_DEVICE_POWER_STATUS,
    ReportPowerStatus = cec_opcode::REPORT_POWER_STATUS,
    FeatureAbort = cec_opcode::FEATURE_ABORT,
    Abort = cec_opcode::ABORT,
    GiveAudioStatus = cec_opcode::GIVE_AUDIO_STATUS,
    GiveSystemAudioModeStatus = cec_opcode::GIVE_SYSTEM_AUDIO_MODE_STATUS,
    ReportAudioStatus = cec_opcode::REPORT_AUDIO_STATUS,
    SetSystemAudioMode = cec_opcode::SET_SYSTEM_AUDIO_MODE,
    SystemAudioModeRequest = cec_opcode::SYSTEM_AUDIO_MODE_REQUEST,
    SystemAudioModeStatus = cec_opcode::SYSTEM_AUDIO_MODE_STATUS,
    SetAudioRate = cec_opcode::SET_AUDIO_RATE,
    ReportShortAudioDescriptors = cec_opcode::REPORT_SHORT_AUDIO_DESCRIPTORS,
    RequestShortAudioDescriptors = cec_opcode::REQUEST_SHORT_AUDIO_DESCRIPTORS,
    StartArc = cec_opcode::START_ARC,
    ReportArcStarted = cec_opcode::REPORT_ARC_STARTED,
    ReportArcEnded = cec_opcode::REPORT_ARC_ENDED,
    RequestArcStart = cec_opcode::REQUEST_ARC_START,
    RequestArcEnd = cec_opcode::REQUEST_ARC_END,
    EndArc = cec_opcode::END_ARC,
    Cdc = cec_opcode::CDC,
    None = cec_opcode::NONE,
}

#[EnumRepr(type = "cec_log_level")]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum LogLevel {
    Error = cec_log_level::CEC_LOG_ERROR,
    Warning = cec_log_level::CEC_LOG_WARNING,
    Notice = cec_log_level::CEC_LOG_NOTICE,
    Traffic = cec_log_level::CEC_LOG_TRAFFIC,
    Debug = cec_log_level::CEC_LOG_DEBUG,
    All = cec_log_level::CEC_LOG_ALL,
}

#[EnumRepr(type = "cec_bus_device_status")]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum BusDeviceStatus {
    Unknown = cec_bus_device_status::UNKNOWN,
    Present = cec_bus_device_status::PRESENT,
    NotPresent = cec_bus_device_status::NOT_PRESENT,
    HandledByLibcec = cec_bus_device_status::HANDLED_BY_LIBCEC,
}

#[EnumRepr(type = "cec_vendor_id")]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum VendorId {
    Toshiba = cec_vendor_id::TOSHIBA,
    Samsung = cec_vendor_id::SAMSUNG,
    Denon = cec_vendor_id::DENON,
    Marantz = cec_vendor_id::MARANTZ,
    Loewe = cec_vendor_id::LOEWE,
    Onkyo = cec_vendor_id::ONKYO,
    Medion = cec_vendor_id::MEDION,
    Toshiba2 = cec_vendor_id::TOSHIBA2,
    Apple = cec_vendor_id::APPLE,
    PulseEight = cec_vendor_id::PULSE_EIGHT,
    HarmanKardon2 = cec_vendor_id::HARMAN_KARDON2,
    Google = cec_vendor_id::GOOGLE,
    Akai = cec_vendor_id::AKAI,
    Aoc = cec_vendor_id::AOC,
    Panasonic = cec_vendor_id::PANASONIC,
    Philips = cec_vendor_id::PHILIPS,
    Daewoo = cec_vendor_id::DAEWOO,
    Yamaha = cec_vendor_id::YAMAHA,
    Grundig = cec_vendor_id::GRUNDIG,
    Pioneer = cec_vendor_id::PIONEER,
    Lg = cec_vendor_id::LG,
    Sharp = cec_vendor_id::SHARP,
    Sony = cec_vendor_id::SONY,
    Broadcom = cec_vendor_id::BROADCOM,
    Sharp2 = cec_vendor_id::SHARP2,
    Vizio = cec_vendor_id::VIZIO,
    Benq = cec_vendor_id::BENQ,
    HarmanKardon = cec_vendor_id::HARMAN_KARDON,
    Unknown = cec_vendor_id::UNKNOWN,
}

#[EnumRepr(type = "cec_adapter_type")]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum AdapterType {
    Unknown = cec_adapter_type::UNKNOWN,
    P8External = cec_adapter_type::P8_EXTERNAL,
    P8Daughterboard = cec_adapter_type::P8_DAUGHTERBOARD,
    Rpi = cec_adapter_type::RPI,
    Tda995x = cec_adapter_type::TDA995x,
    Exynos = cec_adapter_type::EXYNOS,
    Linux = cec_adapter_type::LINUX,
    Aocec = cec_adapter_type::AOCEC,
    Imx = cec_adapter_type::IMX,
}

#[EnumRepr(type = "libcec_version")]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum LibraryVersion {
    Current = libcec_version::CURRENT,
}

#[EnumRepr(type = "libcec_alert")]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Alert {
    ServiceDevice = libcec_alert::SERVICE_DEVICE,
    ConnectionLost = libcec_alert::CONNECTION_LOST,
    PermissionError = libcec_alert::PERMISSION_ERROR,
    PortBusy = libcec_alert::PORT_BUSY,
    PhysicalAddressError = libcec_alert::PHYSICAL_ADDRESS_ERROR,
    TvPollFailed = libcec_alert::TV_POLL_FAILED,
}

#[EnumRepr(type = "libcec_parameter_type")]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum ParameterType {
    String = libcec_parameter_type::STRING,
    Unknown = libcec_parameter_type::UNKOWN,
}

impl TryFrom<c_int> for LogicalAddress {
    type Error = TryFromLogicalAddressesError;

    fn try_from(value: c_int) -> Result<Self, Self::Error> {
        let x = match value {
            -1 => LogicalAddress::Unknown,
            0 => LogicalAddress::Tv,
            1 => LogicalAddress::Recordingdevice1,
            2 => LogicalAddress::Recordingdevice2,
            3 => LogicalAddress::Tuner1,
            4 => LogicalAddress::Playbackdevice1,
            5 => LogicalAddress::Audiosystem,
            6 => LogicalAddress::Tuner2,
            7 => LogicalAddress::Tuner3,
            8 => LogicalAddress::Playbackdevice2,
            9 => LogicalAddress::Recordingdevice3,
            10 => LogicalAddress::Tuner4,
            11 => LogicalAddress::Playbackdevice3,
            12 => LogicalAddress::Reserved1,
            13 => LogicalAddress::Reserved2,
            14 => LogicalAddress::Freeuse,
            15 => LogicalAddress::Unregistered,
            _ => return Err(TryFromLogicalAddressesError::InvalidPrimaryAddress),
        };

        Ok(x)
    }
}
