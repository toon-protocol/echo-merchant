# The payer: one Nanode on the public internet, tagged toon-demo.
#
# Only instances that carry the toon-demo tag AND whose id this demo recorded
# are ever deleted. Nothing else on the account is touched.

linode_create() {
    command -v linode-cli >/dev/null || die "needs linode-cli (uv tool install linode-cli)"
    [ -n "${LINODE_CLI_TOKEN:-${LINODE_ACCESS_TOKEN:-}}" ] || die "set LINODE_CLI_TOKEN (or LINODE_ACCESS_TOKEN)"
    if [ -s "$STATE/linode-id" ]; then echo "payer already created: $(cat "$STATE/linode-id")"; return; fi
    [ -s "$SSH_KEY" ] || ssh-keygen -q -t ed25519 -N '' -C toon-demo -f "$SSH_KEY"
    # A root password is required by the API; it is random, never shown, and
    # unused: login is by the key above.
    local pass; pass=$(head -c 48 /dev/urandom | base64 | tr -d '/+=' | head -c 40)
    local out
    out=$(lin linodes create --type "$LINODE_TYPE" --region "$LINODE_REGION" --image "$LINODE_IMAGE" \
        --label "$LINODE_LABEL-$(date +%s)" --tags "$LINODE_TAG" \
        --root_pass "$pass" --authorized_keys "$(cat "$SSH_KEY.pub")" \
        --json) || die "linode create failed"
    unset pass
    printf '%s' "$out" | python3 -c 'import json,sys; print(json.load(sys.stdin)[0]["id"])' > "$STATE/linode-id"
    printf '%s' "$out" | python3 -c 'import json,sys; print(json.load(sys.stdin)[0]["ipv4"][0])' > "$STATE/linode-ip"
    echo "payer created: id $(cat "$STATE/linode-id"), $(payer_ip), $LINODE_TYPE in $LINODE_REGION, tag $LINODE_TAG"
}

linode_wait() {
    local i
    for i in $(seq 1 60); do
        ssh_payer true 2>/dev/null && { echo "payer reachable over SSH"; return; }
        sleep 5
    done
    die "the payer did not come up"
}

# Delete what this demo made: the recorded id, checked against the tag, plus
# any other instance that carries the tag and the demo's label prefix.
linode_destroy() {
    local ids id tags label
    ids=$( { cat "$STATE/linode-id" 2>/dev/null; \
        lin linodes list --tags "$LINODE_TAG" --text --no-headers --format id,label | \
        awk -v p="$LINODE_LABEL-" 'index($2, p) == 1 {print $1}'; } | sort -u)
    for id in $ids; do
        tags=$(lin linodes view "$id" --text --no-headers --format tags 2>/dev/null) || continue
        label=$(lin linodes view "$id" --text --no-headers --format label 2>/dev/null)
        case ",$tags," in
            *",$LINODE_TAG,"*) ;;
            *) echo "skip $id ($label): not tagged $LINODE_TAG"; continue ;;
        esac
        lin linodes delete "$id" && echo "payer deleted: id $id ($label)"
    done
    rm -f "$STATE/linode-id" "$STATE/linode-ip" "$STATE/known_hosts"
    echo "instances tagged $LINODE_TAG now: $(lin linodes list --tags "$LINODE_TAG" --text --no-headers --format id | wc -l)"
}
