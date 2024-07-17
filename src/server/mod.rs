pub mod manager;
pub mod protocols;

// The Server module consists of a manager and all available layers that provide access to internal services.
//
// Manager:
// The Manager module requires a DeviceManagerHandler, which will be used to forward all incoming requests.
// This allows the Manager to receive and process requests from RestAPI and WebSocket methods.
// The requests are forwarded to the DeviceManager using the server's AppData, which holds a clone of the DeviceManager's Handler and will provide the responses.
//
// Front-end:
// The frontend provides access to REST API documentation through {address}/docs with a Swagger interface and the API specifications.
//
// RestAPI:
// The REST API will have a default route and versioned routes.
// To keep the application stable through updates, users can use {address}/v{x}/route.
//
// WebSocket:
// WebSocket is provided via the {address}/ws route.
// Users can use the following queries:
//     ?filter="some_desired_string_to_use_regex"
//     ?device-number="00000000-0000-0000-b9c0-f5752d453eb3" // The UUID provided by the source of the device created
// Otherwise, if they are not defined, the WebSocket channel will receive all available messages.
// All operations made through REST API and WebSocket routes will be broadcast to all clients subscribed to device-number=null (default),
// except for errors, which are forwarded directly to the requester.
