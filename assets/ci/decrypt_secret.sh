#!/bin/bash

cd "$(dirname "$0")"/../..
for i in "ck3" "eu4" "imperator"; do
    gpg --quiet --batch --yes --decrypt --passphrase="$TOKEN_PASSPHRASE" --output assets/tokens/$i.txt assets/tokens/$i.txt.gpg
done;
