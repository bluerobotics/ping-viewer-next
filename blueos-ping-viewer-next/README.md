# BlueOS's Ping Viewer Next Extension

## Instructions

Access the extensions manager and install with the following parameters:

Extensions Manager:

```shell
blueos.local/tools/extensions-manager
```

Parameters:

```shell
raulelektron.ping-viewer-next

Ping Viewer Next

0.0.0

{
  "ExposedPorts": {
    "6060/tcp": {}
  },
  "HostConfig": {
    "Privileged": true,
    "PortBindings": {
      "6060/tcp": [
        {
          "HostPort": ""
        }
      ]
    },
    "NetworkMode": "host"
  }
}
```
