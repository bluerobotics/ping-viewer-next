/// The `devices` module defines the pattern for managing devices and their handlers.
/// It includes the `Device` and `DeviceHandler` structures.
///
/// The `DeviceHandler` can forward requests defined in the `PingRequest` enum.
pub mod devices;

/// The `manager` module provides the `Manager` and `ManagerHandler` structures.
///
/// The `Manager` can handle requests from multiple threads. The `ManagerHandler`
/// is capable of forwarding requests defined in the `Request` enum and can create
/// devices as needed.
///
/// If a device is stopped or encounters an error during execution, it can be recovered
/// and made available again.
pub mod manager;
