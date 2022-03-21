if [[ -z "$1" ]]; then
  echo "Expected a version number"
  exit 1
fi
docker push enokson/msg-store-http-server
docker push enokson/msg-store-http-server:$1
