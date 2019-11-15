//! IEEE 802.15.4 radio driver for nRF52

use core::cell::Cell;
use core::convert::TryFrom;
use kernel;
use kernel::common::cells::{OptionalCell, TakeCell};
use kernel::common::registers::{register_bitfields, ReadOnly, ReadWrite, WriteOnly};
use kernel::common::StaticRef;
use kernel::hil::radio::{self, PowerClient};
use kernel::hil::time::Alarm;
use kernel::ReturnCode;

use crate::ppi;
use nrf5x;
use nrf5x::constants::TxPower;

//extern crate::net;
//use capsules;
//use capsules::net::ieee802154::{
//    FrameType, FrameVersion, Header, KeyId, MacAddress, PanID, Security, SecurityLevel,
//};
//use capsules::net::ieee802154::Header;

const RADIO_BASE: StaticRef<RadioRegisters> =
    unsafe { StaticRef::new(0x40001000 as *const RadioRegisters) };

pub const IEEE802154_PAYLOAD_LENGTH: usize = 255;
pub const IEEE802154_BACKOFF_PERIOD: usize = 320; //microseconds = 20 symbols
pub const IEEE802154_ACK_TIME: usize = 512; //microseconds = 32 symbols
pub const IEEE802154_MAX_POLLING_ATTEMPTS: u8 = 4;
pub const IEEE802154_MIN_BE: u8 = 3;
pub const IEEE802154_MAX_BE: u8 = 5;
pub const RAM_S0_BYTES: usize = 1;
pub const RAM_LEN_BITS: usize = 8;
pub const RAM_S1_BITS: usize = 0;
pub const PREBUF_LEN_BYTES: usize = 2;

// IEEEStd 802.15.4-2011 Section 8.1.2.2
// Frequency is 2405 + 5 * (k - 11) MHz, where k = 11, 12, ... , 26.
#[derive(PartialEq, Debug, Copy, Clone)]
pub enum RadioChannel {
    DataChannel11 = 5,
    DataChannel12 = 10,
    DataChannel13 = 15,
    DataChannel14 = 20,
    DataChannel15 = 25,
    DataChannel16 = 30,
    DataChannel17 = 35,
    DataChannel18 = 40,
    DataChannel19 = 45,
    DataChannel20 = 50,
    DataChannel21 = 55,
    DataChannel22 = 60,
    DataChannel23 = 65,
    DataChannel24 = 70,
    DataChannel25 = 75,
    DataChannel26 = 80,
}

impl RadioChannel {
    pub fn get_channel_index(&self) -> u8 {
        match *self {
            RadioChannel::DataChannel11 => 11,
            RadioChannel::DataChannel12 => 12,
            RadioChannel::DataChannel13 => 13,
            RadioChannel::DataChannel14 => 14,
            RadioChannel::DataChannel15 => 15,
            RadioChannel::DataChannel16 => 16,
            RadioChannel::DataChannel17 => 17,
            RadioChannel::DataChannel18 => 18,
            RadioChannel::DataChannel19 => 19,
            RadioChannel::DataChannel20 => 20,
            RadioChannel::DataChannel21 => 21,
            RadioChannel::DataChannel22 => 22,
            RadioChannel::DataChannel23 => 23,
            RadioChannel::DataChannel24 => 24,
            RadioChannel::DataChannel25 => 25,
            RadioChannel::DataChannel26 => 26,
        }
    }
}

impl TryFrom<u8> for RadioChannel {
    type Error = ();

    fn try_from(val: u8) -> Result<RadioChannel, ()> {
        match val {
            11 => Ok(RadioChannel::DataChannel11),
            12 => Ok(RadioChannel::DataChannel12),
            13 => Ok(RadioChannel::DataChannel13),
            14 => Ok(RadioChannel::DataChannel14),
            15 => Ok(RadioChannel::DataChannel15),
            16 => Ok(RadioChannel::DataChannel16),
            17 => Ok(RadioChannel::DataChannel17),
            18 => Ok(RadioChannel::DataChannel18),
            19 => Ok(RadioChannel::DataChannel19),
            20 => Ok(RadioChannel::DataChannel20),
            21 => Ok(RadioChannel::DataChannel21),
            22 => Ok(RadioChannel::DataChannel22),
            23 => Ok(RadioChannel::DataChannel23),
            24 => Ok(RadioChannel::DataChannel24),
            25 => Ok(RadioChannel::DataChannel25),
            26 => Ok(RadioChannel::DataChannel26),
            _ => Err(()),
        }
    }
}

#[repr(C)]
struct RadioRegisters {
    /// Enable Radio in TX mode
    /// - Address: 0x000 - 0x004
    task_txen: WriteOnly<u32, Task::Register>,
    /// Enable Radio in RX mode
    /// - Address: 0x004 - 0x008
    task_rxen: WriteOnly<u32, Task::Register>,
    /// Start Radio
    /// - Address: 0x008 - 0x00c
    task_start: WriteOnly<u32, Task::Register>,
    /// Stop Radio
    /// - Address: 0x00c - 0x010
    task_stop: WriteOnly<u32, Task::Register>,
    /// Disable Radio
    /// - Address: 0x010 - 0x014
    task_disable: WriteOnly<u32, Task::Register>,
    /// Start the RSSI and take one single sample of the receive signal strength
    /// - Address: 0x014- 0x018
    task_rssistart: WriteOnly<u32, Task::Register>,
    /// Stop the RSSI measurement
    /// - Address: 0x018 - 0x01c
    task_rssistop: WriteOnly<u32, Task::Register>,
    /// Start the bit counter
    /// - Address: 0x01c - 0x020
    task_bcstart: WriteOnly<u32, Task::Register>,
    /// Stop the bit counter
    /// - Address: 0x020 - 0x024
    task_bcstop: WriteOnly<u32, Task::Register>,
    /// Reserved
    _reserved1: [u32; 2],
    /// Stop the bit counter
    /// - Address: 0x02c - 0x030
    task_ccastart: WriteOnly<u32, Task::Register>,
    /// Stop the bit counter
    /// - Address: 0x030 - 0x034
    task_ccastop: WriteOnly<u32, Task::Register>,
    /// Reserved
    _reserved2: [u32; 51],
    /// Radio has ramped up and is ready to be started
    /// - Address: 0x100 - 0x104
    event_ready: ReadWrite<u32, Event::Register>,
    /// Address sent or received
    /// - Address: 0x104 - 0x108
    event_address: ReadWrite<u32, Event::Register>,
    /// Packet payload sent or received
    /// - Address: 0x108 - 0x10c
    event_payload: ReadWrite<u32, Event::Register>,
    /// Packet sent or received
    /// - Address: 0x10c - 0x110
    event_end: ReadWrite<u32, Event::Register>,
    /// Radio has been disabled
    /// - Address: 0x110 - 0x114
    event_disabled: ReadWrite<u32, Event::Register>,
    /// A device address match occurred on the last received packet
    /// - Address: 0x114 - 0x118
    event_devmatch: ReadWrite<u32>,
    /// No device address match occurred on the last received packet
    /// - Address: 0x118 - 0x11c
    event_devmiss: ReadWrite<u32, Event::Register>,
    /// Sampling of receive signal strength complete
    /// - Address: 0x11c - 0x120
    event_rssiend: ReadWrite<u32, Event::Register>,
    /// Reserved
    _reserved3: [u32; 2],
    /// Bit counter reached bit count value
    /// - Address: 0x128 - 0x12c
    event_bcmatch: ReadWrite<u32, Event::Register>,
    /// Reserved
    _reserved4: [u32; 1],
    /// Packet received with CRC ok
    /// - Address: 0x130 - 0x134
    event_crcok: ReadWrite<u32, Event::Register>,
    /// Packet received with CRC error
    /// - Address: 0x134 - 0x138
    crcerror: ReadWrite<u32, Event::Register>,
    /// IEEE 802.15.4 length field received
    /// - Address: 0x138 - 0x13c
    event_framestart: ReadWrite<u32, Event::Register>,
    /// Reserved
    _reserved5: [u32; 2],
    /// Wireless medium in idle - clear to send
    /// - Address: 0x144-0x148
    event_ccaidle: ReadWrite<u32, Event::Register>,
    /// Wireless medium busy - do not send
    /// - Address: 0x148-0x14c
    event_ccabusy: ReadWrite<u32, Event::Register>,
    /// Reserved
    _reserved6: [u32; 45],
    /// Shortcut register
    /// - Address: 0x200 - 0x204
    shorts: ReadWrite<u32, Shortcut::Register>,
    /// Reserved
    _reserved7: [u32; 64],
    /// Enable interrupt
    /// - Address: 0x304 - 0x308
    intenset: ReadWrite<u32, Interrupt::Register>,
    /// Disable interrupt
    /// - Address: 0x308 - 0x30c
    intenclr: ReadWrite<u32, Interrupt::Register>,
    /// Reserved
    _reserved8: [u32; 61],
    /// CRC status
    /// - Address: 0x400 - 0x404
    crcstatus: ReadOnly<u32, Event::Register>,
    /// Reserved
    _reserved9: [u32; 1],
    /// Received address
    /// - Address: 0x408 - 0x40c
    rxmatch: ReadOnly<u32, ReceiveMatch::Register>,
    /// CRC field of previously received packet
    /// - Address: 0x40c - 0x410
    rxcrc: ReadOnly<u32, ReceiveCrc::Register>,
    /// Device address match index
    /// - Address: 0x410 - 0x414
    dai: ReadOnly<u32, DeviceAddressIndex::Register>,
    /// Reserved
    _reserved10: [u32; 60],
    /// Packet pointer
    /// - Address: 0x504 - 0x508
    packetptr: ReadWrite<u32, PacketPointer::Register>,
    /// Frequency
    /// - Address: 0x508 - 0x50c
    frequency: ReadWrite<u32, Frequency::Register>,
    /// Output power
    /// - Address: 0x50c - 0x510
    txpower: ReadWrite<u32, TransmitPower::Register>,
    /// Data rate and modulation
    /// - Address: 0x510 - 0x514
    mode: ReadWrite<u32, Mode::Register>,
    /// Packet configuration register 0
    /// - Address 0x514 - 0x518
    pcnf0: ReadWrite<u32, PacketConfiguration0::Register>,
    /// Packet configuration register 1
    /// - Address: 0x518 - 0x51c
    pcnf1: ReadWrite<u32, PacketConfiguration1::Register>,
    /// Base address 0
    /// - Address: 0x51c - 0x520
    base0: ReadWrite<u32, BaseAddress::Register>,
    /// Base address 1
    /// - Address: 0x520 - 0x524
    base1: ReadWrite<u32, BaseAddress::Register>,
    /// Prefix bytes for logical addresses 0-3
    /// - Address: 0x524 - 0x528
    prefix0: ReadWrite<u32, Prefix0::Register>,
    /// Prefix bytes for logical addresses 4-7
    /// - Address: 0x528 - 0x52c
    prefix1: ReadWrite<u32, Prefix1::Register>,
    /// Transmit address select
    /// - Address: 0x52c - 0x530
    txaddress: ReadWrite<u32, TransmitAddress::Register>,
    /// Receive address select
    /// - Address: 0x530 - 0x534
    rxaddresses: ReadWrite<u32, ReceiveAddresses::Register>,
    /// CRC configuration
    /// - Address: 0x534 - 0x538
    crccnf: ReadWrite<u32, CrcConfiguration::Register>,
    /// CRC polynomial
    /// - Address: 0x538 - 0x53c
    crcpoly: ReadWrite<u32, CrcPolynomial::Register>,
    /// CRC initial value
    /// - Address: 0x53c - 0x540
    crcinit: ReadWrite<u32, CrcInitialValue::Register>,
    /// Reserved
    _reserved11: [u32; 1],
    /// Interframe spacing in microseconds
    /// - Address: 0x544 - 0x548
    tifs: ReadWrite<u32, InterFrameSpacing::Register>,
    /// RSSI sample
    /// - Address: 0x548 - 0x54c
    rssisample: ReadWrite<u32, RssiSample::Register>,
    /// Reserved
    _reserved12: [u32; 1],
    /// Current radio state
    /// - Address: 0x550 - 0x554
    state: ReadOnly<u32, State::Register>,
    /// Data whitening initial value
    /// - Address: 0x554 - 0x558
    datawhiteiv: ReadWrite<u32, DataWhiteIv::Register>,
    /// Reserved
    _reserved13: [u32; 2],
    /// Bit counter compare
    /// - Address: 0x560 - 0x564
    bcc: ReadWrite<u32, BitCounterCompare::Register>,
    /// Reserved
    _reserved14: [u32; 39],
    /// Device address base segments
    /// - Address: 0x600 - 0x620
    dab: [ReadWrite<u32, DeviceAddressBase::Register>; 8],
    /// Device address prefix
    /// - Address: 0x620 - 0x640
    dap: [ReadWrite<u32, DeviceAddressPrefix::Register>; 8],
    /// Device address match configuration
    /// - Address: 0x640 - 0x644
    dacnf: ReadWrite<u32, DeviceAddressMatch::Register>,
    /// MAC header Search Pattern Configuration
    /// - Address: 0x644 - 0x648
    mhrmatchconf: ReadWrite<u32, MACHeaderSearch::Register>,
    /// MAC Header Search Pattern Mask
    /// - Address: 0x648 - 0x64C
    mhrmatchmas: ReadWrite<u32, MACHeaderMask::Register>,
    /// Reserved
    _reserved15: [u32; 1],
    /// Radio mode configuration register
    /// - Address: 0x650 - 0x654
    modecnf0: ReadWrite<u32, RadioModeConfig::Register>,
    /// Reserved
    _reserved16: [u32; 6],
    /// Clear Channel Assesment (CCA) control register
    /// - Address: 0x66C - 0x670
    ccactrl: ReadWrite<u32, CCAControl::Register>,
    /// Reserved
    _reserved17: [u32; 611],
    /// Peripheral power control
    /// - Address: 0xFFC - 0x1000
    power: ReadWrite<u32, Task::Register>,
}

register_bitfields! [u32,
    /// Task register
    Task [
        /// Enable task
        ENABLE OFFSET(0) NUMBITS(1)
    ],
    /// Event register
    Event [
        /// Ready event
        READY OFFSET(0) NUMBITS(1)
    ],
    /// Shortcut register
    Shortcut [
        /// Shortcut between READY event and START task
        READY_START OFFSET(0) NUMBITS(1),
        /// Shortcut between END event and DISABLE task
        END_DISABLE OFFSET(1) NUMBITS(1),
        /// Shortcut between DISABLED event and TXEN task
        DISABLED_TXEN OFFSET(2) NUMBITS(1),
        /// Shortcut between DISABLED event and RXEN task
        DISABLED_RXEN OFFSET(3) NUMBITS(1),
        /// Shortcut between ADDRESS event and RSSISTART task
        ADDRESS_RSSISTART OFFSET(4) NUMBITS(1),
        /// Shortcut between END event and START task
        END_START OFFSET(5) NUMBITS(1),
        /// Shortcut between ADDRESS event and BCSTART task
        ADDRESS_BCSTART OFFSET(6) NUMBITS(1),
        /// Shortcut between DISABLED event and RSSISTOP task
        DISABLED_RSSISTOP OFFSET(8) NUMBITS(1)
    ],
    /// Interrupt register
    Interrupt [
        /// READY event
        READY OFFSET(0) NUMBITS(1),
        /// ADDRESS event
        ADDRESS OFFSET(1) NUMBITS(1),
        /// PAYLOAD event
        PAYLOAD OFFSET(2) NUMBITS(1),
        /// END event
        END OFFSET(3) NUMBITS(1),
        /// DISABLED event
        DISABLED OFFSET(4) NUMBITS(1),
        /// DEVMATCH event
        DEVMATCH OFFSET(5) NUMBITS(1),
        /// DEVMISS event
        DEVMISS OFFSET(6) NUMBITS(1),
        /// RSSIEND event
        RSSIEND OFFSET(7) NUMBITS(1),
        /// BCMATCH event
        BCMATCH OFFSET(10) NUMBITS(1),
        /// CRCOK event
        CRCOK OFFSET(12) NUMBITS(1),
        /// CRCERROR event
        CRCERROR OFFSET(13) NUMBITS(1),
        /// CCAIDLE event
        FRAMESTART OFFSET(14) NUMBITS(1),
        /// CCAIDLE event
        CCAIDLE OFFSET(17) NUMBITS(1),
        /// CCABUSY event
        CCABUSY OFFSET(18) NUMBITS(1)
    ],
    /// Receive match register
    ReceiveMatch [
        /// Logical address of which previous packet was received
        MATCH OFFSET(0) NUMBITS(3)
    ],
    /// Received CRC register
    ReceiveCrc [
        /// CRC field of previously received packet
        CRC OFFSET(0) NUMBITS(24)
    ],
    /// Device address match index register
    DeviceAddressIndex [
        /// Device address match index
        /// Index (n) of device address, see DAB\[n\] and DAP\[n\], that got an
        /// address match
        INDEX OFFSET(0) NUMBITS(3)
    ],
    /// Packet pointer register
    PacketPointer [
        /// Packet address to be used for the next transmission or reception. When transmitting, the packet pointed to by this
        /// address will be transmitted and when receiving, the received packet will be written to this address. This address is a byte
        /// aligned ram address.
        POINTER OFFSET(0) NUMBITS(32)
    ],
    /// Frequency register
    Frequency [
        /// Radio channel frequency
        /// Frequency = 2400 + FREQUENCY (MHz)
        FREQUENCY OFFSET(0) NUMBITS(7) [],
        /// Channel map selection.
        /// Channel map between 2400 MHZ .. 2500 MHZ
        MAP OFFSET(8) NUMBITS(1) [
            DEFAULT = 0,
            LOW = 1
        ]
    ],
    /// Transmitting power register
    TransmitPower [
        /// Radio output power
        POWER OFFSET(0) NUMBITS(8) [
            POS4DBM = 4,
            POS3DBM = 3,
            ODBM = 0,
            NEG4DBM = 0xfc,
            NEG8DBM = 0xf8,
            NEG12DBM = 0xf4,
            NEG16DBM = 0xf0,
            NEG20DBM = 0xec,
            NEG40DBM = 0xd8
        ]
    ],
    /// Data rate and modulation register
    Mode [
        /// Radio data rate and modulation setting.
        /// The radio supports Frequency-shift Keying (FSK) modulation
        MODE OFFSET(0) NUMBITS(4) [
            NRF_1MBIT = 0,
            NRF_2MBIT = 1,
            NRF_250KBIT = 2,
            BLE_1MBIT = 3,
            BLE_2MBIT = 4,
            BLE_LR125KBIT = 5,
            BLE_LR500KBIT = 6,
            IEEE802154_250KBIT = 15
        ]
    ],
    /// Packet configuration register 0
    PacketConfiguration0 [
        /// Length on air of LENGTH field in number of bits
        LFLEN OFFSET(0) NUMBITS(4) [],
        /// Length on air of S0 field in number of bytes
        S0LEN OFFSET(8) NUMBITS(1) [],
        /// Length on air of S1 field in number of bits.
        S1LEN OFFSET(16) NUMBITS(4) [],
        /// Include or exclude S1 field in RAM
        S1INCL OFFSET(20) NUMBITS(1) [
            AUTOMATIC = 0,
            INCLUDE = 1
        ],
        /// Length of preamble on air. Decision point: TASKS_START task
        PLEN OFFSET(24) NUMBITS(2) [
            EIGHT = 0,
            SIXTEEN = 1,
            THIRTYTWOZEROS = 2,
            LONGRANGE = 3
        ],
        CRCINC OFFSET(26) NUMBITS(1) [
            EXCLUDE = 0,
            INCLUDE = 1
        ]
    ],
    /// Packet configuration register 1
    PacketConfiguration1 [
        /// Maximum length of packet payload
        MAXLEN OFFSET(0) NUMBITS(8) [],
        /// Static length in number of bytes
        STATLEN OFFSET(8) NUMBITS(8) [],
        /// Base address length in number of bytes
        BALEN OFFSET(16) NUMBITS(3) [],
        /// On air endianness
        ENDIAN OFFSET(24) NUMBITS(1) [
            LITTLE = 0,
            BIG = 1
        ],
        /// Enable or disable packet whitening
        WHITEEN OFFSET(25) NUMBITS(1) [
            DISABLED = 0,
            ENABLED = 1
        ]
    ],
    /// Radio base address register
    BaseAddress [
        /// BASE0 or BASE1
        BASE OFFSET(0) NUMBITS(32)
    ],
    /// Radio prefix0 registers
    Prefix0 [
        /// Address prefix 0
        AP0 OFFSET(0) NUMBITS(8),
        /// Address prefix 1
        AP1 OFFSET(8) NUMBITS(8),
        /// Address prefix 2
        AP2 OFFSET(16) NUMBITS(8),
        /// Address prefix 3
        AP3 OFFSET(24) NUMBITS(8)
    ],
    /// Radio prefix0 registers
    Prefix1 [
        /// Address prefix 4
        AP4 OFFSET(0) NUMBITS(8),
        /// Address prefix 5
        AP5 OFFSET(8) NUMBITS(8),
        /// Address prefix 6
        AP6 OFFSET(16) NUMBITS(8),
        /// Address prefix 7
        AP7 OFFSET(24) NUMBITS(8)
    ],
    /// Transmit address register
    TransmitAddress [
        /// Logical address to be used when transmitting a packet
        ADDRESS OFFSET(0) NUMBITS(3)
    ],
    /// Receive addresses register
    ReceiveAddresses [
        /// Enable or disable reception on logical address 0-7
        ADDRESS OFFSET(0) NUMBITS(8)
    ],
    /// CRC configuration register
    CrcConfiguration [
        /// CRC length in bytes
        LEN OFFSET(0) NUMBITS(2) [
            DISABLED = 0,
            ONE = 1,
            TWO = 2,
            THREE = 3
        ],
        /// Include or exclude packet field from CRC calculation
        SKIPADDR OFFSET(8) NUMBITS(2) [
            INCLUDE = 0,
            EXCLUDE = 1,
            IEEE802154 = 2
        ]
    ],
    /// CRC polynomial register
    CrcPolynomial [
        /// CRC polynomial
        CRCPOLY OFFSET(0) NUMBITS(24)
    ],
    /// CRC initial value register
    CrcInitialValue [
       /// Initial value for CRC calculation
       CRCINIT OFFSET(0) NUMBITS(24)
    ],
    /// Inter Frame Spacing in us register
    InterFrameSpacing [
        /// Inter Frame Spacing in us
        /// Inter frame space is the time interval between two consecutive packets. It is defined as the time, in micro seconds, from the
        /// end of the last bit of the previous packet to the start of the first bit of the subsequent packet
        TIFS OFFSET(0) NUMBITS(8)
    ],
    /// RSSI sample register
    RssiSample [
        /// RSSI sample result
        RSSISAMPLE OFFSET(0) NUMBITS(7)
    ],
    /// Radio state register
    State [
        /// Current radio state
        STATE OFFSET(0) NUMBITS(4) [
            DISABLED = 0,
            RXRU = 1,
            RXIDLE = 2,
            RX = 3,
            RXDISABLED = 4,
            TXRU = 9,
            TXIDLE = 10,
            TX = 11,
            TXDISABLED = 12
        ]
    ],
    /// Data whitening initial value register
    DataWhiteIv [
        /// Data whitening initial value. Bit 6 is hard-wired to '1', writing '0'
        /// to it has no effect, and it will always be read back and used by the device as '1'
        DATEWHITEIV OFFSET(0) NUMBITS(7)
    ],
    /// Bit counter compare register
    BitCounterCompare [
        /// Bit counter compare
        BCC OFFSET(0) NUMBITS(32)
    ],
    /// Device address base register
    DeviceAddressBase [
        /// Device address base 0-7
        DAB OFFSET(0) NUMBITS(32)
    ],
    /// Device address prefix register
    DeviceAddressPrefix [
        /// Device address prefix 0-7
        DAP OFFSET(0) NUMBITS(32)
    ],
    /// Device address match configuration register
    DeviceAddressMatch [
        /// Enable or disable device address matching on 0-7
        ENA OFFSET(0) NUMBITS(8),
        /// TxAdd for device address 0-7
        TXADD OFFSET(8) NUMBITS(8)
    ],
    MACHeaderSearch [
        CONFIG OFFSET(0) NUMBITS(32)
    ],
    MACHeaderMask [
        PATTERN OFFSET(0) NUMBITS(32)
    ],
    CCAControl [
        CCAMODE OFFSET(0) NUMBITS(3) [
            ED_MODE = 0,
            CARRIER_MODE = 1,
            CARRIER_AND_ED_MODE = 2,
            CARRIER_OR_ED_MODE = 3,
            ED_MODE_TEST_1 = 4
        ],
        CCAEDTHRESH OFFSET(8) NUMBITS(8) [],
        CCACORRTHRESH OFFSET(16) NUMBITS(8) [],
        CCACORRCNT OFFSET(24) NUMBITS(8) []
    ],
    /// Radio mode configuration register
    RadioModeConfig [
        /// Radio ramp-up time
        RU OFFSET(0) NUMBITS(1) [
            DEFAULT = 0,
            FAST = 1
        ],
        /// Default TX value
        /// Specifies what the RADIO will transmit when it is not started, i.e. between:
        /// RADIO.EVENTS_READY and RADIO.TASKS_START
        /// RADIO.EVENTS_END and RADIO.TASKS_START
        DTX OFFSET(8) NUMBITS(2) [
            B1 = 0,
            B0 = 1,
            CENTER = 2
        ]
    ]
];

pub struct Radio {
    registers: StaticRef<RadioRegisters>,
    tx_power: Cell<TxPower>,
    rx_client: OptionalCell<&'static dyn radio::RxClient>,
    tx_client: OptionalCell<&'static dyn radio::TxClient>,
    tx_buf: TakeCell<'static, [u8]>,
    rx_buf: TakeCell<'static, [u8]>,
    addr: Cell<u16>,
    addr_long: Cell<[u8; 8]>,
    pan: Cell<u16>,
    cca_count: Cell<u8>,
    cca_be: Cell<u8>,
    random_nonce: Cell<u32>,
    channel: Cell<RadioChannel>,
    transmitting: Cell<bool>,
}

pub static mut RADIO: Radio = Radio::new();

impl Radio {
    pub const fn new() -> Radio {
        Radio {
            registers: RADIO_BASE,
            tx_power: Cell::new(TxPower::ZerodBm),
            rx_client: OptionalCell::empty(),
            tx_client: OptionalCell::empty(),
            tx_buf: TakeCell::empty(),
            rx_buf: TakeCell::empty(),
            addr: Cell::new(0),
            addr_long: Cell::new([0x00; 8]),
            pan: Cell::new(0),
            cca_count: Cell::new(0),
            cca_be: Cell::new(0),
            random_nonce: Cell::new(0xDEADBEEF),
            channel: Cell::new(RadioChannel::DataChannel11),
            transmitting: Cell::new(false),
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.registers
            .mode
            .matches_all(Mode::MODE::IEEE802154_250KBIT)
    }

    fn rx(&self) {
        let regs = &*self.registers;
        regs.event_ready.write(Event::READY::CLEAR);

        if self.transmitting.get() {
            let tbuf = self
                .tx_buf
                .take()
                .expect("Radio TX Buffer produced an invalid result when setting the DMA pointer.");

            self.tx_buf.replace(self.set_dma_ptr(tbuf));
        } else {
            let rbuf = self
                .rx_buf
                .take()
                .expect("Radio RX Buffer produced an invalid result when setting the DMA pointer.");
            self.rx_buf.replace(self.set_dma_ptr(rbuf));
        }

        regs.task_rxen.write(Task::ENABLE::SET);

        self.enable_interrupts();
    }

    fn set_rx_address(&self) {
        let regs = &*self.registers;
        regs.rxaddresses.write(ReceiveAddresses::ADDRESS.val(1));
    }

    fn set_tx_address(&self) {
        let regs = &*self.registers;
        regs.txaddress.write(TransmitAddress::ADDRESS.val(0));
    }

    fn radio_on(&self) {
        let regs = &*self.registers;
        // reset and enable power
        regs.power.write(Task::ENABLE::CLEAR);
        regs.power.write(Task::ENABLE::SET);
    }

    fn radio_off(&self) {
        let regs = &*self.registers;
        regs.power.write(Task::ENABLE::CLEAR);
    }

    fn set_tx_power(&self) {
        let regs = &*self.registers;
        regs.txpower.set(self.tx_power.get() as u32);
    }

    fn set_dma_ptr(&self, buffer: &'static mut [u8]) -> &'static mut [u8] {
        let regs = &*self.registers;
        regs.packetptr.set(buffer.as_ptr() as u32);
        buffer
    }

    // TODO: Theres an additional step for 802154 rx/tx handling
    #[inline(never)]
    pub fn handle_interrupt(&self) {
        let regs = &*self.registers;
        self.disable_all_interrupts();

        if regs.event_ready.is_set(Event::READY) {
            regs.event_ready.write(Event::READY::CLEAR);
            regs.event_end.write(Event::READY::CLEAR);
            if self.transmitting.get() && regs.state.get() == nrf5x::constants::RADIO_STATE_RXIDLE {
                if self.cca_count.get() > 0 {
                    unsafe {
                        ppi::PPI.disable(ppi::Channel::CH21::SET);
                    }
                }
                regs.task_ccastart.write(Task::ENABLE::SET);
            } else {
                regs.task_start.write(Task::ENABLE::SET);
            }
        }

        if regs.event_framestart.is_set(Event::READY) {
            regs.event_framestart.write(Event::READY::CLEAR);
        }

        //   IF we receive the go ahead (channel is clear)
        // THEN start the transmit part of the radio
        if regs.event_ccaidle.is_set(Event::READY) {
            regs.event_ccaidle.write(Event::READY::CLEAR);
            regs.task_txen.write(Task::ENABLE::SET)
        }

        if regs.event_ccabusy.is_set(Event::READY) {
            regs.event_ccabusy.write(Event::READY::CLEAR);
            //need to back off for a period of time outlined
            //in the IEEE 802.15.4 standard (see Figure 69 in
            //section 7.5.1.4 The CSMA-CA algorithm of the
            //standard).
            if self.cca_count.get() < IEEE802154_MAX_POLLING_ATTEMPTS {
                self.cca_count.set(self.cca_count.get() + 1);
                self.cca_be.set(self.cca_be.get() + 1);
                let backoff_periods = self.random_nonce() & ((1 << self.cca_be.get()) - 1);
                unsafe {
                    ppi::PPI.enable(ppi::Channel::CH21::SET);
                    nrf5x::timer::TIMER0
                        .set_alarm(backoff_periods * (IEEE802154_BACKOFF_PERIOD as u32));
                }
            } else {
                self.transmitting.set(false);
                //if we are transmitting, the CRCstatus check is always going to be an error
                let result = ReturnCode::EBUSY;
                //TODO: Acked is flagged as false until I get around to fixing it.
                self.tx_client
                    .map(|client| {
                        let tbuf = self.tx_buf.take().expect("TX Buffer produced error when sending it back to the requestor after the channel was busy.");
                        client.send_done(tbuf, false, result)
                    });
            }

            regs.event_ready.write(Event::READY::CLEAR);
            regs.task_disable.write(Task::ENABLE::SET);
            self.enable_interrupts();
        }

        // tx or rx finished!
        if regs.event_end.is_set(Event::READY) {
            regs.event_end.write(Event::READY::CLEAR);

            let result = if regs.crcstatus.is_set(Event::READY) {
                ReturnCode::SUCCESS
            } else {
                ReturnCode::FAIL
            };

            match regs.state.get() {
                nrf5x::constants::RADIO_STATE_TXRU
                | nrf5x::constants::RADIO_STATE_TXIDLE
                | nrf5x::constants::RADIO_STATE_TXDISABLE
                | nrf5x::constants::RADIO_STATE_TX => {
                    self.transmitting.set(false);
                    //if we are transmitting, the CRCstatus check is always going to be an error
                    let result = ReturnCode::SUCCESS;
                    //TODO: Acked is flagged as false until I get around to fixing it.
                    self.tx_client
                        .map(|client|{
                        let tbuf = self.tx_buf.take().expect("TX Buffer produced error when sending it back to the requestor after successful transmission.");

                         client.send_done(tbuf, false, result)
                    });
                }
                nrf5x::constants::RADIO_STATE_RXRU
                | nrf5x::constants::RADIO_STATE_RXIDLE
                | nrf5x::constants::RADIO_STATE_RXDISABLE
                | nrf5x::constants::RADIO_STATE_RX => {
                    self.rx_client.map(|client| {
                        let rbuf = self.rx_buf.take().expect(
                            "RX Buffer produced error when sending received packet to requestor",
                        );

                        let frame_len = rbuf[1] as usize - radio::MFR_SIZE;
                        // Length is: S0 (1 Byte) + Length (1 Byte) + S1 (0 Bytes) + Payload
                        // And because the length field is directly read from the packet
                        // We need to add 2 to length to get the total length

                        client.receive(rbuf, frame_len, regs.crcstatus.get() == 1, result)
                    });
                }
                // Radio state - Disabled
                _ => (),
            }
            self.radio_off();
            self.radio_initialize(self.channel.get());
            self.rx();
        }
        self.enable_interrupts();
    }

    pub fn enable_interrupts(&self) {
        let regs = &*self.registers;
        regs.intenset.write(
            Interrupt::READY::SET
                + Interrupt::CCAIDLE::SET
                + Interrupt::CCABUSY::SET
                + Interrupt::END::SET
                + Interrupt::FRAMESTART::SET,
        );
    }

    pub fn enable_interrupt(&self, intr: u32) {
        let regs = &*self.registers;
        regs.intenset.set(intr);
    }

    pub fn clear_interrupt(&self, intr: u32) {
        let regs = &*self.registers;
        regs.intenclr.set(intr);
    }

    pub fn disable_all_interrupts(&self) {
        let regs = &*self.registers;
        // disable all possible interrupts
        regs.intenclr.set(0xffffffff);
    }

    fn radio_initialize(&self, _channel: RadioChannel) {
        self.radio_on();

        self.ieee802154_set_channel_rate();

        self.ieee802154_set_packet_config();

        self.ieee802154_set_rampup_mode();

        self.ieee802154_set_crc_config();
        self.ieee802154_set_cca_config();

        self.ieee802154_set_tx_power();

        self.ieee802154_set_channel_freq(self.channel.get());

        self.set_tx_address();
        self.set_rx_address();

        self.rx();
    }

    // IEEE802.15.4 SPECIFICATION Section 6.20.12.5 of the NRF52840 Datasheet
    fn ieee802154_set_crc_config(&self) {
        let regs = &*self.registers;
        regs.crccnf
            .write(CrcConfiguration::LEN::TWO + CrcConfiguration::SKIPADDR::IEEE802154);
        regs.crcinit.set(nrf5x::constants::RADIO_CRCINIT_IEEE802154);
        regs.crcpoly.set(nrf5x::constants::RADIO_CRCPOLY_IEEE802154);
    }

    fn ieee802154_set_rampup_mode(&self) {
        let regs = &*self.registers;
        regs.modecnf0
            .write(RadioModeConfig::RU::FAST + RadioModeConfig::DTX::CENTER);
    }

    fn ieee802154_set_cca_config(&self) {
        let regs = &*self.registers;
        regs.ccactrl.write(
            CCAControl::CCAMODE.val(nrf5x::constants::IEEE802154_CCA_MODE)
                + CCAControl::CCAEDTHRESH.val(nrf5x::constants::IEEE802154_CCA_ED_THRESH)
                + CCAControl::CCACORRTHRESH.val(nrf5x::constants::IEEE802154_CCA_CORR_THRESH)
                + CCAControl::CCACORRCNT.val(nrf5x::constants::IEEE802154_CCA_CORR_CNT),
        );
    }

    // Packet configuration
    // Settings taken from OpenThread nrf_radio_init() in
    // openthread/third_party/NordicSemiconductor/drivers/radio/nrf_802154_core.c
    //
    fn ieee802154_set_packet_config(&self) {
        let regs = &*self.registers;

        // sets the header of PDU TYPE to 1 byte
        // sets the header length to 1 byte
        regs.pcnf0.write(
            PacketConfiguration0::LFLEN.val(8)
                + PacketConfiguration0::S0LEN.val(1)
                + PacketConfiguration0::S1LEN::CLEAR
                + PacketConfiguration0::S1INCL::CLEAR
                + PacketConfiguration0::PLEN::THIRTYTWOZEROS
                + PacketConfiguration0::CRCINC::INCLUDE,
        );

        regs.pcnf1.write(
            //PacketConfiguration1::WHITEEN::ENABLED
            PacketConfiguration1::ENDIAN::LITTLE
                //+ PacketConfiguration1::BALEN.val(3)
                + PacketConfiguration1::STATLEN::CLEAR
                + PacketConfiguration1::MAXLEN.val(nrf5x::constants::RADIO_PAYLOAD_LENGTH as u32),
        );
    }

    fn ieee802154_set_channel_rate(&self) {
        let regs = &*self.registers;
        regs.mode.write(Mode::MODE::IEEE802154_250KBIT);
    }

    fn ieee802154_set_channel_freq(&self, channel: RadioChannel) {
        let regs = &*self.registers;
        regs.frequency
            .write(Frequency::FREQUENCY.val(channel as u32));
    }

    fn ieee802154_set_tx_power(&self) {
        self.set_tx_power();
    }

    pub fn startup(&self) -> ReturnCode {
        self.radio_initialize(self.channel.get());
        ReturnCode::SUCCESS
    }

    // Returns a new pseudo-random number and updates the randomness state.
    //
    // Uses the [Xorshift](https://en.wikipedia.org/wiki/Xorshift) algorithm to
    // produce pseudo-random numbers. Uses the `random_nonce` field to keep
    // state.
    fn random_nonce(&self) -> u32 {
        let mut next_nonce = ::core::num::Wrapping(self.random_nonce.get());
        next_nonce ^= next_nonce << 13;
        next_nonce ^= next_nonce >> 17;
        next_nonce ^= next_nonce << 5;
        self.random_nonce.set(next_nonce.0);
        self.random_nonce.get()
    }
}

impl kernel::hil::radio::Radio for Radio {}

impl kernel::hil::radio::RadioConfig for Radio {
    fn initialize(
        &self,
        _spi_buf: &'static mut [u8],
        _reg_write: &'static mut [u8],
        _reg_read: &'static mut [u8],
    ) -> ReturnCode {
        self.radio_initialize(self.channel.get());
        ReturnCode::SUCCESS
    }

    fn set_power_client(&self, _client: &'static dyn PowerClient) {
        //
    }

    fn reset(&self) -> ReturnCode {
        self.radio_on();
        ReturnCode::SUCCESS
    }
    fn start(&self) -> ReturnCode {
        self.reset();
        ReturnCode::SUCCESS
    }
    fn stop(&self) -> ReturnCode {
        self.radio_off();
        ReturnCode::SUCCESS
    }
    fn is_on(&self) -> bool {
        true
    }
    fn busy(&self) -> bool {
        false
    }

    //#################################################
    ///These methods are holdovers from when the radio HIL was mostly to an external
    ///module over an interface
    //#################################################

    //fn set_power_client(&self, client: &'static radio::PowerClient){

    //}
    /// Commit the config calls to hardware, changing the address,
    /// PAN ID, TX power, and channel to the specified values, issues
    /// a callback to the config client when done.
    fn config_commit(&self) {
        self.radio_off();
        self.radio_initialize(self.channel.get());
    }

    fn set_config_client(&self, _client: &'static dyn radio::ConfigClient) {}

    //#################################################
    /// Accessors
    //#################################################

    fn get_address(&self) -> u16 {
        self.addr.get()
    }

    fn get_address_long(&self) -> [u8; 8] {
        self.addr_long.get()
    }

    /// The 16-bit PAN ID
    fn get_pan(&self) -> u16 {
        self.pan.get()
    }
    /// The transmit power, in dBm
    fn get_tx_power(&self) -> i8 {
        self.tx_power.get() as i8
    }
    /// The 802.15.4 channel
    fn get_channel(&self) -> u8 {
        self.channel.get().get_channel_index()
    }

    //#################################################
    /// Mutators
    //#################################################

    fn set_address(&self, addr: u16) {
        self.addr.set(addr);
    }

    fn set_address_long(&self, addr: [u8; 8]) {
        self.addr_long.set(addr);
    }

    fn set_pan(&self, id: u16) {
        self.pan.set(id);
    }

    fn set_channel(&self, chan: u8) -> ReturnCode {
        match RadioChannel::try_from(chan) {
            Err(_) => ReturnCode::ENOSUPPORT,
            Ok(res) => {
                self.channel.set(res);
                ReturnCode::SUCCESS
            }
        }
    }

    fn set_tx_power(&self, tx_power: i8) -> ReturnCode {
        // Convert u8 to TxPower
        match nrf5x::constants::TxPower::try_from(tx_power as u8) {
            // Invalid transmitting power, propogate error
            Err(_) => ReturnCode::ENOSUPPORT,
            // Valid transmitting power, propogate success
            Ok(res) => {
                self.tx_power.set(res);
                ReturnCode::SUCCESS
            }
        }
    }
}

impl kernel::hil::radio::RadioData for Radio {
    fn set_receive_client(&self, client: &'static dyn radio::RxClient, buffer: &'static mut [u8]) {
        self.rx_client.set(client);
        self.rx_buf.replace(buffer);
    }

    fn set_receive_buffer(&self, buffer: &'static mut [u8]) {
        self.rx_buf.replace(buffer);
    }

    fn set_transmit_client(&self, client: &'static dyn radio::TxClient) {
        self.tx_client.set(client);
    }

    fn transmit(
        &self,
        buf: &'static mut [u8],
        frame_len: usize,
    ) -> (ReturnCode, Option<&'static mut [u8]>) {
        if self.tx_buf.is_some() || self.transmitting.get() {
            return (ReturnCode::EBUSY, Some(buf));
        } else if radio::PSDU_OFFSET + frame_len >= buf.len() {
            // Not enough room for CRC
            return (ReturnCode::ESIZE, Some(buf));
        }

        buf[RAM_S0_BYTES] = frame_len as u8;

        self.tx_buf.replace(buf);

        self.transmitting.set(true);

        self.cca_count.set(0);
        self.cca_be.set(IEEE802154_MIN_BE);

        self.radio_off();
        self.radio_initialize(self.channel.get());

        //self.enable_interrupts();
        (ReturnCode::SUCCESS, None)
    }
}
