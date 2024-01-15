#!/usr/bin/env sh
sudo docker cp ./target/release/app tesla_pika:/app/
sudo docker cp ./configs/in_docker.json tesla_pika:/app/configs/app.json
sudo docker cp ./web/build tesla_pika:/app/web/build
