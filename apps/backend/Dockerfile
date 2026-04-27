# syntax=docker/dockerfile:1.7

FROM node:22-alpine AS build
WORKDIR /app
COPY package*.json ./
RUN npm ci
COPY . .
RUN node ace build --ignore-ts-errors

FROM node:22-alpine AS runtime
ARG APP_UID=1001
ARG APP_GID=1001
RUN addgroup -g ${APP_GID} -S webapp \
 && adduser -S -D -H -u ${APP_UID} -G webapp -s /sbin/nologin webapp

WORKDIR /app
ENV NODE_ENV=production \
    HOST=0.0.0.0 \
    PORT=3333 \
    LOG_LEVEL=info

COPY --from=build --chown=webapp:webapp /app/build ./
RUN cd /app && npm ci --omit=dev && npm cache clean --force

USER webapp:webapp
EXPOSE 3333
HEALTHCHECK --interval=30s --timeout=3s CMD wget -qO- http://127.0.0.1:3333/healthz || exit 1
CMD ["node", "bin/server.js"]
