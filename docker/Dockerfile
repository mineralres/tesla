FROM ubuntu:22.04

LABEL maintainer="pangxiaobing@gmail.com"

ENV APP=/app \
	PATH=${APP}/bin:${PATH}

WORKDIR ${APP}

COPY ./app /app/
COPY ./entrypoint.sh /entrypoint.sh
COPY ./configs/ /app/configs

#ENTRYPOINT ["/entrypoint.sh"]
RUN apt-get update && apt-get install -y \
    ca-certificates \
    && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists /var/cache/apt/archives

EXPOSE 3600

CMD ["/app/app", "--config", "/app/configs/app.json"]