import base64
import os
import sys
import time

import jwt
import requests

API = "https://api.appstoreconnect.apple.com/v1/bundleIds"


def token():
    key = base64.b64decode(os.environ["PRIVATE_KEY_BASE64"]).decode()
    now = int(time.time())
    claims = {"iss": os.environ["ISSUER_ID"], "iat": now, "exp": now + 600, "aud": "appstoreconnect-v1"}
    return jwt.encode(claims, key, algorithm="ES256", headers={"kid": os.environ["KEY_ID"]})


def main():
    identifier = os.environ["BUNDLE_IDENTIFIER"]
    headers = {"Authorization": f"Bearer {token()}"}
    existing = requests.get(API, headers=headers, params={"filter[identifier]": identifier}, timeout=30)
    existing.raise_for_status()
    if any(item["attributes"]["identifier"] == identifier for item in existing.json()["data"]):
        print(f"{identifier} is already registered")
        return
    body = {
        "data": {
            "type": "bundleIds",
            "attributes": {"identifier": identifier, "name": "randomon", "platform": "MAC_OS"},
        }
    }
    created = requests.post(API, headers=headers, json=body, timeout=30)
    if created.status_code >= 400:
        print(created.text, file=sys.stderr)
        created.raise_for_status()
    print(f"Registered {identifier}")


main()
