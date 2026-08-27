# 纯运行时 Dockerfile：不进行任何编译。
# server 二进制与 admin-web 静态产物由仓库根目录 ./build-docker.sh 在本地
# 构建后复制进构建上下文（server/filehub-server 与 web/）。
FROM nginx:1.27-alpine

RUN apk add --no-cache jq openssl \
 && rm -f /etc/nginx/conf.d/default.conf \
 && mkdir -p /etc/filehub /data /data/files \
 && chmod 700 /data

COPY nginx.conf /etc/nginx/filehub.conf.tpl
COPY entrypoint.sh /usr/local/bin/filehub-entrypoint
COPY server/filehub-server /usr/local/bin/filehub-server
COPY web/ /usr/share/nginx/html/

RUN chmod +x /usr/local/bin/filehub-entrypoint

EXPOSE 80

HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
  CMD wget -q -O /dev/null http://127.0.0.1/healthz || exit 1

ENTRYPOINT ["/usr/local/bin/filehub-entrypoint"]
