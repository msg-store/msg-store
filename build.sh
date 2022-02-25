# docker build -t msg-store-armv7-builder-2 .
docker build -t msg-store-http-server builds/http-server
docker build -t msg-store-http-server:armv7 builds/http-server/armv7
