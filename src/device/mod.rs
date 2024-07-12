pub mod devices;
pub mod manager;

// The Device module consists of two main modules: devices and manager.
//
// Manager:
// The Manager module includes two primary structures: Manager and its Handler.
// This design allows the Manager to receive and process requests from multiple, distinct threads.
// The ManagerHandler can forward requests defined in the Request enum, creating a Device if necessary.
// If a device is stopped or encounters an error during execution, the user can recover the device and make it available again.
//
// Device:
// Each device follows the same pattern, consisting of a Device and its Handler.
// The DeviceHandler can forward requests defined in the PingRequest enum.
