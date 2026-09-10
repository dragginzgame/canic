export DO_NOT_TRACK=1
export XDG_CONFIG_HOME=/tmp/toko-feedback-fleet-identity/config
export XDG_DATA_HOME=/tmp/toko-feedback-fleet-identity/data
cd /tmp/toko-feedback-fleet
source scripts/dev/local-dev-functions.sh
[[ "$ROOT_DIR" == /tmp/toko-feedback-fleet && "$LOCAL_GATEWAY_PORT" == 18014 ]]
FLEET_NAME=toko-miner-retained-qualification
FLEET_SOURCE="${RUNTIME_DIR}/${FLEET_NAME}.policy.toml"
FLEET_SEED="${RUNTIME_DIR}/${FLEET_NAME}.estate.toml"
FLEET_DESIRED="${RUNTIME_DIR}/${FLEET_NAME}.toml"
FLEET_ENSURE_DIRECTORY="${ROOT_DIR}/.canic/fleet-ensure/${ENVIRONMENT}/${FLEET_NAME}"
RUNTIME_DIR="${ROOT_DIR}/.tools/retained-qualification"
mkdir -p "$RUNTIME_DIR"
release=43e2cf9b9258fea38f9c10d000621fe28a3f179a0baa2f72ddacd70c22e24b4f
artifacts="${ROOT_DIR}/.canic/release-builds/${release}/artifacts"
select_local_identity
