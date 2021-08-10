FROM gcr.io/dexpa-175115/broxus/rust-runtime:1

COPY target/release/ton-wallet-api /app/application
COPY migrations /app/migrations
COPY entrypoint.sh /app/entrypoint.sh

USER runuser

EXPOSE 9000

ENTRYPOINT ["/app/entrypoint.sh"]
