# BlueOS's Ping Viewer Next Extension

## Instructions

### Manual Instalation

Access the extensions manager and install with the following parameters:

Extensions Manager:

```shell
blueos.local/tools/extensions-manager
```

Parameters:

```shell
bluerobotics.ping-viewer-next

Ping Viewer Next

1.0.0-beta.6

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

### Cockpit Widgets

For each connected ping device, provides a widget accessible through Cockpit as an
[Automatic External Iframe](https://blueos.cloud/cockpit/docs/latest/usage/advanced/#automatic-external-iframes).

Requires BlueOS >= 1.4.
