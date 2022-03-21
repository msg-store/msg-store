
if [[ -z "$1" ]]; then
  echo "Expected a version number"
  exit 1
fi

echo "Found $1"

# Build the latest docker container
docker build -t enokson/msg-store-http-server builds/http-server
# Add a tag to the container
docker tag enokson/msg-store-http-server enokson/msg-store-http-server:$1