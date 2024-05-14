FROM public.ecr.aws/docker/library/alpine:latest AS builder
LABEL authors="hotfoxy"
RUN apk add --no-cache rust cargo pkgconfig openssl-dev
COPY . /rusty-ws-server
WORKDIR /rusty-ws-server
RUN cargo build --release && cp target/release/rusty-ws-server rusty-ws-server
RUN rm -rf target
RUN rm -rf src

FROM public.ecr.aws/docker/library/alpine:latest
LABEL authors="hotfoxy"
RUN apk add --no-cache libgcc
COPY --from=builder /rusty-ws-server /rusty-ws-server
WORKDIR /rusty-ws-server
CMD ["./rusty-ws-server"]