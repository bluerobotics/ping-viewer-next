FROM alpine:3.18

ARG TARGETARCH

RUN apk add --no-cache bash shadow

COPY ./blueos-ping-viewer-next/files/ping-viewer-next.$TARGETARCH /app/ping-viewer-next
COPY ./blueos-ping-viewer-next/files/entrypoint.sh /

RUN addgroup -g 1000 pingviewer && adduser -G pingviewer -u 1000 pingviewer -D

# Add docker configuration
LABEL version="1.0.0-beta.6"
LABEL permissions='{\
  "ExposedPorts": {\
    "6060/tcp": {}\
  },\
  "HostConfig": {\
    "Privileged": true,\
    "NetworkMode": "host",\
    "Binds": [\
       "/var/logs/blueos/extensions/ping-viewer-next:/app/logs",\
       "/usr/blueos/extensions/ping-viewer-next/recordings:/app/recordings",\
       "/usr/blueos/extensions/ping-viewer-next/settings:/app/settings"\
    ]\
  }\
}'
LABEL authors='[\
    {\
        "name": "Blue Robotics",\
        "email": "software@bluerobotics.com"\
    }\
]'
LABEL company='{\
  "about": "Control PingProtocol based hardware using webservices",\
  "name": "Blue Robotics",\
  "email": "support@bluerobotics.com"\
}'
LABEL readme="https://raw.githubusercontent.com/bluerobotics/ping-viewer-next/refs/heads/master/blueos-ping-viewer-next/README.md"
LABEL type="device-integration"
LABEL tags='[\
  "sonar",\
  "ping-protocol"\
]'

ENTRYPOINT ["/entrypoint.sh"]
