//! A generic USB client layer managing control requests
//!
//! It responds to control requests and forwards bulk/interrupt transfers to the above layer.

use super::descriptors::Buffer64;
use super::descriptors::ConfigurationDescriptor;
use super::descriptors::Descriptor;
use super::descriptors::DescriptorType;
use super::descriptors::DeviceDescriptor;
use super::descriptors::EndpointDescriptor;
use super::descriptors::HIDDescriptor;
use super::descriptors::InterfaceDescriptor;
use super::descriptors::LanguagesDescriptor;
use super::descriptors::Recipient;
use super::descriptors::ReportDescriptor;
use super::descriptors::SetupData;
use super::descriptors::StandardRequest;
use super::descriptors::StringDescriptor;
use super::descriptors::TransferDirection;
use core::cell::Cell;
use core::cmp::min;
use kernel::hil;

const DESCRIPTOR_BUFLEN: usize = 64;

const N_ENDPOINTS: usize = 3;

pub struct ClientCtrl<'a, 'b, C: 'a> {
    // The hardware controller
    controller: &'a C,

    // State for tracking each endpoint
    state: [Cell<State>; N_ENDPOINTS],

    // A 64-byte buffer for the control endpoint
    ctrl_buffer: Buffer64,

    // Storage for composing responses to device-descriptor requests
    descriptor_storage: [Cell<u8>; DESCRIPTOR_BUFLEN],

    // Descriptors to reply to control requests
    device_descriptor: DeviceDescriptor,

    // For now we only support one configuration...
    configuration_descriptor: ConfigurationDescriptor,

    // ...with only one interface
    interface_descriptor: InterfaceDescriptor,

    // A list of endpoints for the configuration
    endpoint_descriptors: &'b [EndpointDescriptor],

    // A HID descriptor for the configuration, if any
    hid_descriptor: Option<&'b HIDDescriptor<'b>>,

    // A report descriptor for the configuration, if any
    report_descriptor: Option<&'b ReportDescriptor<'b>>,

    // Supported language (only one for now)
    language: &'b [u16; 1],

    // Strings
    strings: &'b [&'b str],
}

#[derive(Copy, Clone)]
enum State {
    Init,

    /// We are doing a Control In transfer of some data
    /// in self.descriptor_storage, with the given extent
    /// remaining to send
    CtrlIn(usize, usize),

    /// We will accept data from the host
    CtrlOut,

    SetAddress,
}

impl Default for State {
    fn default() -> Self {
        State::Init
    }
}

impl<C: hil::usb::UsbController> ClientCtrl<'a, 'b, C> {
    pub fn new(
        controller: &'a C,
        device_descriptor: DeviceDescriptor,
        mut configuration_descriptor: ConfigurationDescriptor,
        mut interface_descriptor: InterfaceDescriptor,
        endpoint_descriptors: &'b [EndpointDescriptor],
        hid_descriptor: Option<&'b HIDDescriptor<'b>>,
        report_descriptor: Option<&'b ReportDescriptor<'b>>,
        language: &'b [u16; 1],
        strings: &'b [&'b str],
    ) -> Self {
        // Tweak the configuration/interface descriptors for the given endpoints.
        interface_descriptor.num_endpoints = endpoint_descriptors.len() as u8;

        configuration_descriptor.num_interfaces = 1;
        configuration_descriptor.related_descriptor_length = interface_descriptor.size()
            + endpoint_descriptors.iter().map(|d| d.size()).sum::<usize>()
            + hid_descriptor.map_or(0, |d| d.size());

        ClientCtrl {
            controller: controller,
            state: Default::default(),
            ctrl_buffer: Default::default(),
            // For the moment, the Default trait is not implemented for arrays of length > 32, and
            // the Cell type is not Copy, so we have to initialize each element manually.
            descriptor_storage: [
                Default::default(),
                Default::default(),
                Default::default(),
                Default::default(),
                Default::default(),
                Default::default(),
                Default::default(),
                Default::default(),
                Default::default(),
                Default::default(),
                Default::default(),
                Default::default(),
                Default::default(),
                Default::default(),
                Default::default(),
                Default::default(),
                Default::default(),
                Default::default(),
                Default::default(),
                Default::default(),
                Default::default(),
                Default::default(),
                Default::default(),
                Default::default(),
                Default::default(),
                Default::default(),
                Default::default(),
                Default::default(),
                Default::default(),
                Default::default(),
                Default::default(),
                Default::default(),
                Default::default(),
                Default::default(),
                Default::default(),
                Default::default(),
                Default::default(),
                Default::default(),
                Default::default(),
                Default::default(),
                Default::default(),
                Default::default(),
                Default::default(),
                Default::default(),
                Default::default(),
                Default::default(),
                Default::default(),
                Default::default(),
                Default::default(),
                Default::default(),
                Default::default(),
                Default::default(),
                Default::default(),
                Default::default(),
                Default::default(),
                Default::default(),
                Default::default(),
                Default::default(),
                Default::default(),
                Default::default(),
                Default::default(),
                Default::default(),
                Default::default(),
                Default::default(),
            ],
            device_descriptor,
            configuration_descriptor,
            interface_descriptor,
            endpoint_descriptors,
            hid_descriptor,
            report_descriptor,
            language,
            strings,
        }
    }

    #[inline]
    pub fn controller(&self) -> &'a C {
        self.controller
    }

    #[inline]
    fn descriptor_buf(&'a self) -> &'a [Cell<u8>] {
        &self.descriptor_storage
    }

    pub fn enable(&self) {
        // Set up the default control endpoint
        self.controller
            .endpoint_set_buffer(0, &self.ctrl_buffer.buf);
        self.controller
            .enable_as_device(hil::usb::DeviceSpeed::Full); // must be Full for Bulk transfers
        self.controller.endpoint_ctrl_out_enable(0);
    }

    pub fn attach(&self) {
        self.controller.attach();
    }

    /// Handle a Control Setup transaction
    pub fn ctrl_setup(&self, endpoint: usize) -> hil::usb::CtrlSetupResult {
        if endpoint != 0 {
            // For now we only support the default Control endpoint
            return hil::usb::CtrlSetupResult::ErrInvalidDeviceIndex;
        }
        SetupData::get(&self.ctrl_buffer.buf).map_or(
            hil::usb::CtrlSetupResult::ErrNoParse,
            |setup_data| {
                let transfer_direction = setup_data.request_type.transfer_direction();
                let recipient = setup_data.request_type.recipient();
                setup_data.get_standard_request().map_or_else(
                    || {
                        // XX: CtrlSetupResult::ErrNonstandardRequest

                        // For now, promiscuously accept vendor data and even supply
                        // a few debugging bytes when host does a read

                        match transfer_direction {
                            TransferDirection::HostToDevice => {
                                self.state[endpoint].set(State::CtrlOut);
                                hil::usb::CtrlSetupResult::Ok
                            }
                            TransferDirection::DeviceToHost => {
                                // Arrange to send some crap back
                                let buf = self.descriptor_buf();
                                buf[0].set(0xa);
                                buf[1].set(0xb);
                                buf[2].set(0xc);
                                self.state[endpoint].set(State::CtrlIn(0, 3));
                                hil::usb::CtrlSetupResult::Ok
                            }
                        }
                    },
                    |request| match recipient {
                        Recipient::Device => self.handle_standard_device_request(endpoint, request),
                        Recipient::Interface => {
                            self.handle_standard_interface_request(endpoint, request)
                        }
                        _ => hil::usb::CtrlSetupResult::ErrGeneric,
                    },
                )
            },
        )
    }

    fn handle_standard_device_request(
        &self,
        endpoint: usize,
        request: StandardRequest,
    ) -> hil::usb::CtrlSetupResult {
        match request {
            StandardRequest::GetDescriptor {
                descriptor_type,
                descriptor_index,
                lang_id,
                requested_length,
            } => {
                match descriptor_type {
                    DescriptorType::Device => match descriptor_index {
                        0 => {
                            let buf = self.descriptor_buf();
                            let len = self.device_descriptor.write_to(buf);

                            let end = min(len, requested_length as usize);
                            self.state[endpoint].set(State::CtrlIn(0, end));
                            hil::usb::CtrlSetupResult::Ok
                        }
                        _ => hil::usb::CtrlSetupResult::ErrInvalidDeviceIndex,
                    },
                    DescriptorType::Configuration => {
                        match descriptor_index {
                            0 => {
                                // Place all the descriptors related to this configuration into a
                                // buffer contiguously.

                                let buf = self.descriptor_buf();
                                let mut len = 0;

                                // A single configuration, with the following interface.
                                len += self.configuration_descriptor.write_to(&buf[len..]);

                                // A single interface, with the following descriptors and endpoints.
                                len += self.interface_descriptor.write_to(&buf[len..]);

                                // HID descriptor, if any.
                                if let Some(dh) = self.hid_descriptor {
                                    len += dh.write_to(&buf[len..]);
                                }

                                // Endpoints.
                                for de in self.endpoint_descriptors {
                                    len += de.write_to(&buf[len..]);
                                }

                                let end = min(len, requested_length as usize);
                                self.state[endpoint].set(State::CtrlIn(0, end));
                                hil::usb::CtrlSetupResult::Ok
                            }
                            _ => hil::usb::CtrlSetupResult::ErrInvalidConfigurationIndex,
                        }
                    }
                    DescriptorType::Interface => match descriptor_index {
                        0 => {
                            let buf = self.descriptor_buf();
                            let len = self.interface_descriptor.write_to(buf);

                            let end = min(len, requested_length as usize);
                            self.state[endpoint].set(State::CtrlIn(0, end));
                            hil::usb::CtrlSetupResult::Ok
                        }
                        _ => hil::usb::CtrlSetupResult::ErrInvalidInterfaceIndex,
                    },
                    DescriptorType::String => {
                        if let Some(len) = match descriptor_index {
                            0 => {
                                let buf = self.descriptor_buf();
                                let d = LanguagesDescriptor {
                                    langs: self.language,
                                };
                                let len = d.write_to(buf);
                                Some(len)
                            }
                            i if i > 0
                                && (i as usize) <= self.strings.len()
                                && lang_id == self.language[0] =>
                            {
                                let buf = self.descriptor_buf();
                                let d = StringDescriptor {
                                    string: self.strings[i as usize - 1],
                                };
                                let len = d.write_to(buf);
                                Some(len)
                            }
                            _ => None,
                        } {
                            let end = min(len, requested_length as usize);
                            self.state[endpoint].set(State::CtrlIn(0, end));
                            hil::usb::CtrlSetupResult::Ok
                        } else {
                            hil::usb::CtrlSetupResult::ErrInvalidStringIndex
                        }
                    }
                    DescriptorType::DeviceQualifier => {
                        // We are full-speed only, so we must
                        // respond with a request error
                        hil::usb::CtrlSetupResult::ErrNoDeviceQualifier
                    }
                    _ => hil::usb::CtrlSetupResult::ErrUnrecognizedDescriptorType,
                } // match descriptor_type
            }
            StandardRequest::SetAddress { device_address } => {
                // Load the address we've been assigned ...
                self.controller.set_address(device_address);

                // ... and when this request gets to the Status stage we will actually enable the
                // address.
                self.state[endpoint].set(State::SetAddress);
                hil::usb::CtrlSetupResult::OkSetAddress
            }
            StandardRequest::SetConfiguration { .. } => {
                // We have been assigned a particular configuration: fine!
                hil::usb::CtrlSetupResult::Ok
            }
            _ => hil::usb::CtrlSetupResult::ErrUnrecognizedRequestType,
        }
    }

    fn handle_standard_interface_request(
        &self,
        endpoint: usize,
        request: StandardRequest,
    ) -> hil::usb::CtrlSetupResult {
        match request {
            StandardRequest::GetDescriptor {
                descriptor_type,
                // TODO: use the descriptor index
                descriptor_index: _,
                // TODO: use the language ID?
                lang_id: _,
                requested_length,
            } => match descriptor_type {
                DescriptorType::HID => {
                    if let Some(desc) = self.hid_descriptor {
                        let buf = self.descriptor_buf();
                        let len = desc.write_to(buf);
                        let end = min(len, requested_length as usize);
                        self.state[endpoint].set(State::CtrlIn(0, end));
                        hil::usb::CtrlSetupResult::Ok
                    } else {
                        hil::usb::CtrlSetupResult::ErrGeneric
                    }
                }
                DescriptorType::Report => {
                    if let Some(desc) = self.report_descriptor {
                        let buf = self.descriptor_buf();
                        let len = desc.write_to(buf);
                        let end = min(len, requested_length as usize);
                        self.state[endpoint].set(State::CtrlIn(0, end));
                        hil::usb::CtrlSetupResult::Ok
                    } else {
                        hil::usb::CtrlSetupResult::ErrGeneric
                    }
                }
                _ => hil::usb::CtrlSetupResult::ErrGeneric,
            },
            _ => hil::usb::CtrlSetupResult::ErrGeneric,
        }
    }

    /// Handle a Control In transaction
    pub fn ctrl_in(&self, endpoint: usize) -> hil::usb::CtrlInResult {
        match self.state[endpoint].get() {
            State::CtrlIn(start, end) => {
                let len = end.saturating_sub(start);
                if len > 0 {
                    let packet_bytes = min(self.ctrl_buffer.buf.len(), len);
                    let packet = &self.descriptor_storage[start..start + packet_bytes];
                    let buf = &self.ctrl_buffer.buf;

                    // Copy a packet into the endpoint buffer
                    for (i, b) in packet.iter().enumerate() {
                        buf[i].set(b.get());
                    }

                    let start = start + packet_bytes;
                    let len = end.saturating_sub(start);
                    let transfer_complete = len == 0;

                    self.state[endpoint].set(State::CtrlIn(start, end));

                    hil::usb::CtrlInResult::Packet(packet_bytes, transfer_complete)
                } else {
                    hil::usb::CtrlInResult::Packet(0, true)
                }
            }
            _ => hil::usb::CtrlInResult::Error,
        }
    }

    /// Handle a Control Out transaction
    pub fn ctrl_out(&self, endpoint: usize, _packet_bytes: u32) -> hil::usb::CtrlOutResult {
        match self.state[endpoint].get() {
            State::CtrlOut => {
                // Gamely accept the data
                hil::usb::CtrlOutResult::Ok
            }
            _ => {
                // Bad state
                hil::usb::CtrlOutResult::Halted
            }
        }
    }

    pub fn ctrl_status(&self, _endpoint: usize) {
        // Entered Status stage
    }

    /// Handle the completion of a Control transfer
    pub fn ctrl_status_complete(&self, endpoint: usize) {
        // Control Read: IN request acknowledged
        // Control Write: status sent

        match self.state[endpoint].get() {
            State::SetAddress => {
                self.controller.enable_address();
            }
            _ => {}
        };
        self.state[endpoint].set(State::Init);
    }
}
