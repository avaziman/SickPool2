// use crate::protocol::Protocol;

// // a protocol that also sends notifications (on top of responding to requests)
// pub trait NotifiableProtocol : Protocol {
//     fn poll_notifications(&self) -> Vec<Self::Request>;
// }

// // impl 