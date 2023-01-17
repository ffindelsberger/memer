FROM rust:1.66

COPY . .
COPY vendors/yt-dlp_macos ./target/release/vendors/yt-dlp_macos

# Install ffmpeg to combine audio and video tracks of reddit downloads
RUN apt-get -y update && apt-get -y upgrade && apt-get install -y --no-install-recommends ffmpeg

RUN cargo build --release

CMD ["./target/release/gamers_bot"]


