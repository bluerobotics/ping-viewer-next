FROM alpine:3.18
RUN apk add --no-cache bash shadow

COPY ./blueos-ping-viewer-next/files/ping-viewer-next.* /
COPY ./blueos-ping-viewer-next/files/entrypoint.sh /

RUN mkdir -p /app
RUN chmod +x /entrypoint.sh && \
    if [ "$(uname -m)" = "aarch64" ]; then \
        cp /ping-viewer-next.aarch64 /app/ping-viewer-next; \
    elif [ "$(uname -m)" = "x86_64" ]; then \
        cp /ping-viewer-next.x86_64 /app/ping-viewer-next; \
    else \
        cp /ping-viewer-next.armv7 /app/ping-viewer-next; \
    fi && \
    chmod +x  /app/ping-viewer-next && \
    rm /ping-viewer-next.*
LABEL version="1.0.0-beta.6"

RUN addgroup -g 1000 pingviewer && adduser -G pingviewer -u 1000 pingviewer -D

# Add docker configuration
LABEL permissions='{\
  "ExposedPorts": {\
    "6060/tcp": {}\
  },\
  "HostConfig": {\
    "Privileged": true,\
    "NetworkMode": "host",\
    "Binds": [\
       "/usr/blueos/extensions/ping-viewer-next/logs:/app/logs"\
    ]\
  }\
}'
LABEL authors='[\
    {\
        "name": "Raul Victor Trombin",\
        "email": "raulvtrombin@gmail.com"\
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
