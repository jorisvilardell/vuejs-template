# syntax=docker/dockerfile:1.7

FROM node:22-alpine AS build
WORKDIR /app
COPY package*.json ./
RUN npm ci
COPY . .
RUN npm run build

FROM nginx:1.27-alpine AS runtime

ARG APP_UID=1001
ARG APP_GID=1001
RUN addgroup -g ${APP_GID} -S webapp \
 && adduser -S -D -H -u ${APP_UID} -G webapp -s /sbin/nologin webapp

COPY --chown=webapp:webapp nginx.main.conf /etc/nginx/nginx.conf
COPY --chown=webapp:webapp nginx.conf      /etc/nginx/conf.d/default.conf
COPY --from=build --chown=webapp:webapp /app/dist/ /usr/share/nginx/html/

RUN chown -R webapp:webapp /var/cache/nginx /var/log/nginx /etc/nginx \
 && touch /tmp/nginx.pid \
 && chown webapp:webapp /tmp/nginx.pid

USER webapp:webapp
WORKDIR /usr/share/nginx/html
EXPOSE 8080
HEALTHCHECK --interval=30s --timeout=3s CMD wget -qO- http://127.0.0.1:8080/healthz || exit 1
CMD ["nginx", "-g", "daemon off;"]
