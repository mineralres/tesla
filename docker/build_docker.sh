#!/usr/bin/env sh
cp ../target/release/app .
cp ../scripts/entrypoint.sh .
cp -r ../configs .
sudo docker build . -t mineralres/tesla:version1.0