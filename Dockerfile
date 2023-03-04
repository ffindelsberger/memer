FROM rust:1.66

COPY . .
COPY vendors/yt-dlp_macos ./target/release/vendors/yt-dlp_macos

# Install ffmpeg, is needed by yt-dlp and reddit downloader
RUN apt-get -y update && apt-get -y upgrade && apt-get install -y --no-install-recommends ffmpeg

RUN cargo build --release

CMD ["./target/release/gamers_bot"]


