//! DO NOT EDIT: This file is auto-generated by ftl_idlc.
#![no_std]
#![feature(const_mut_refs)]
#![feature(const_intrinsic_copy)]

use ftl_types::message::HandleOwnership;
use ftl_types::message::MessageBuffer;
use ftl_types::message::MessageDeserialize;
use ftl_types::message::MessageInfo;
use ftl_types::message::MessageSerialize;

pub mod protocols {
    use super::*;

    pub mod autopilot {
        use super::*;

        #[repr(C)]
        pub struct NewclientRequest {
            pub handle: HandleOwnership,
        }

        #[repr(C)]
        struct InlinedPartNewclientRequest {}

        // TODO: static_assert for size

        impl MessageSerialize for NewclientRequest {
            const MSGINFO: MessageInfo = MessageInfo::from_raw(
                (1 << 14) | (1 << 12) | ::core::mem::size_of::<NewclientRequest>() as isize,
            );

            fn serialize(self, buffer: &mut MessageBuffer) {
                // The actual serialization is done in this const fn. This is to
                // ensure the serialization can be done with const operations.
                const fn do_serialize(this: NewclientRequest, buffer: &mut MessageBuffer) {
                    let object = InlinedPartNewclientRequest {};

                    let dst = buffer as *mut _ as *mut InlinedPartNewclientRequest;
                    let src = &object as *const _ as *const InlinedPartNewclientRequest;

                    unsafe {
                        core::ptr::copy_nonoverlapping::<InlinedPartNewclientRequest>(src, dst, 1);
                    }

                    // FIXME: Support multiple handles.
                    debug_assert!(
                        MessageInfo::from_raw(NewclientRequest::MSGINFO.as_raw()).num_handles()
                            <= 1
                    );

                    buffer.handles[0] = this.handle.0;

                    // Don't call destructors on handles transferred to this buffer.
                    core::mem::forget(this);
                }

                do_serialize(self, buffer)
            }
        }

        impl MessageDeserialize for NewclientRequest {
            type Reader<'a> = NewclientRequestReader<'a>;

            fn deserialize<'a>(
                buffer: &'a MessageBuffer,
                msginfo: MessageInfo,
            ) -> Option<NewclientRequestReader<'a>> {
                if msginfo == Self::MSGINFO {
                    Some(NewclientRequestReader { buffer })
                } else {
                    None
                }
            }
        }

        pub struct NewclientRequestReader<'a> {
            #[allow(dead_code)]
            buffer: &'a MessageBuffer,
        }

        impl<'a> NewclientRequestReader<'a> {
            #[allow(dead_code)]
            fn as_ref(&self, buffer: &'a MessageBuffer) -> &'a InlinedPartNewclientRequest {
                unsafe { &*(buffer as *const _ as *const InlinedPartNewclientRequest) }
            }

            pub fn handle(&self) -> ftl_types::handle::HandleId {
                // TODO: return OwnedHandle
                // FIXME: Support multiple handles.
                self.buffer.handles[0]
            }
        }

        #[repr(C)]
        pub struct NewclientReply {}

        #[repr(C)]
        struct InlinedPartNewclientReply {}

        // TODO: static_assert for size

        impl MessageSerialize for NewclientReply {
            const MSGINFO: MessageInfo = MessageInfo::from_raw(
                (2 << 14) | (0 << 12) | ::core::mem::size_of::<NewclientReply>() as isize,
            );

            fn serialize(self, buffer: &mut MessageBuffer) {
                // The actual serialization is done in this const fn. This is to
                // ensure the serialization can be done with const operations.
                const fn do_serialize(this: NewclientReply, buffer: &mut MessageBuffer) {
                    let object = InlinedPartNewclientReply {};

                    let dst = buffer as *mut _ as *mut InlinedPartNewclientReply;
                    let src = &object as *const _ as *const InlinedPartNewclientReply;

                    unsafe {
                        core::ptr::copy_nonoverlapping::<InlinedPartNewclientReply>(src, dst, 1);
                    }

                    // FIXME: Support multiple handles.
                    debug_assert!(
                        MessageInfo::from_raw(NewclientReply::MSGINFO.as_raw()).num_handles() <= 1
                    );

                    // Don't call destructors on handles transferred to this buffer.
                    core::mem::forget(this);
                }

                do_serialize(self, buffer)
            }
        }

        impl MessageDeserialize for NewclientReply {
            type Reader<'a> = NewclientReplyReader<'a>;

            fn deserialize<'a>(
                buffer: &'a MessageBuffer,
                msginfo: MessageInfo,
            ) -> Option<NewclientReplyReader<'a>> {
                if msginfo == Self::MSGINFO {
                    Some(NewclientReplyReader { buffer })
                } else {
                    None
                }
            }
        }

        pub struct NewclientReplyReader<'a> {
            #[allow(dead_code)]
            buffer: &'a MessageBuffer,
        }

        impl<'a> NewclientReplyReader<'a> {
            #[allow(dead_code)]
            fn as_ref(&self, buffer: &'a MessageBuffer) -> &'a InlinedPartNewclientReply {
                unsafe { &*(buffer as *const _ as *const InlinedPartNewclientReply) }
            }
        }
    }

    pub mod ping {
        use super::*;

        #[repr(C)]
        pub struct PingRequest {
            pub int_value1: i32,
        }

        #[repr(C)]
        struct InlinedPartPingRequest {
            pub int_value1: i32,
        }

        // TODO: static_assert for size

        impl MessageSerialize for PingRequest {
            const MSGINFO: MessageInfo = MessageInfo::from_raw(
                (3 << 14) | (0 << 12) | ::core::mem::size_of::<PingRequest>() as isize,
            );

            fn serialize(self, buffer: &mut MessageBuffer) {
                // The actual serialization is done in this const fn. This is to
                // ensure the serialization can be done with const operations.
                const fn do_serialize(this: PingRequest, buffer: &mut MessageBuffer) {
                    let object = InlinedPartPingRequest {
                        int_value1: this.int_value1,
                    };

                    let dst = buffer as *mut _ as *mut InlinedPartPingRequest;
                    let src = &object as *const _ as *const InlinedPartPingRequest;

                    unsafe {
                        core::ptr::copy_nonoverlapping::<InlinedPartPingRequest>(src, dst, 1);
                    }

                    // FIXME: Support multiple handles.
                    debug_assert!(
                        MessageInfo::from_raw(PingRequest::MSGINFO.as_raw()).num_handles() <= 1
                    );

                    // Don't call destructors on handles transferred to this buffer.
                    core::mem::forget(this);
                }

                do_serialize(self, buffer)
            }
        }

        impl MessageDeserialize for PingRequest {
            type Reader<'a> = PingRequestReader<'a>;

            fn deserialize<'a>(
                buffer: &'a MessageBuffer,
                msginfo: MessageInfo,
            ) -> Option<PingRequestReader<'a>> {
                if msginfo == Self::MSGINFO {
                    Some(PingRequestReader { buffer })
                } else {
                    None
                }
            }
        }

        pub struct PingRequestReader<'a> {
            #[allow(dead_code)]
            buffer: &'a MessageBuffer,
        }

        impl<'a> PingRequestReader<'a> {
            #[allow(dead_code)]
            fn as_ref(&self, buffer: &'a MessageBuffer) -> &'a InlinedPartPingRequest {
                unsafe { &*(buffer as *const _ as *const InlinedPartPingRequest) }
            }

            pub fn int_value1(&self) -> i32 {
                let m = self.as_ref(self.buffer);
                m.int_value1
            }
        }

        #[repr(C)]
        pub struct PingReply {
            pub int_value2: i32,
        }

        #[repr(C)]
        struct InlinedPartPingReply {
            pub int_value2: i32,
        }

        // TODO: static_assert for size

        impl MessageSerialize for PingReply {
            const MSGINFO: MessageInfo = MessageInfo::from_raw(
                (4 << 14) | (0 << 12) | ::core::mem::size_of::<PingReply>() as isize,
            );

            fn serialize(self, buffer: &mut MessageBuffer) {
                // The actual serialization is done in this const fn. This is to
                // ensure the serialization can be done with const operations.
                const fn do_serialize(this: PingReply, buffer: &mut MessageBuffer) {
                    let object = InlinedPartPingReply {
                        int_value2: this.int_value2,
                    };

                    let dst = buffer as *mut _ as *mut InlinedPartPingReply;
                    let src = &object as *const _ as *const InlinedPartPingReply;

                    unsafe {
                        core::ptr::copy_nonoverlapping::<InlinedPartPingReply>(src, dst, 1);
                    }

                    // FIXME: Support multiple handles.
                    debug_assert!(
                        MessageInfo::from_raw(PingReply::MSGINFO.as_raw()).num_handles() <= 1
                    );

                    // Don't call destructors on handles transferred to this buffer.
                    core::mem::forget(this);
                }

                do_serialize(self, buffer)
            }
        }

        impl MessageDeserialize for PingReply {
            type Reader<'a> = PingReplyReader<'a>;

            fn deserialize<'a>(
                buffer: &'a MessageBuffer,
                msginfo: MessageInfo,
            ) -> Option<PingReplyReader<'a>> {
                if msginfo == Self::MSGINFO {
                    Some(PingReplyReader { buffer })
                } else {
                    None
                }
            }
        }

        pub struct PingReplyReader<'a> {
            #[allow(dead_code)]
            buffer: &'a MessageBuffer,
        }

        impl<'a> PingReplyReader<'a> {
            #[allow(dead_code)]
            fn as_ref(&self, buffer: &'a MessageBuffer) -> &'a InlinedPartPingReply {
                unsafe { &*(buffer as *const _ as *const InlinedPartPingReply) }
            }

            pub fn int_value2(&self) -> i32 {
                let m = self.as_ref(self.buffer);
                m.int_value2
            }
        }
    }
}
