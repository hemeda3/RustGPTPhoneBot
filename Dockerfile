# Use the specified base image
FROM instrumentisto/rust:beta

# Install necessary dependencies
RUN apt-get update && apt-get install -y \
    build-essential \
    libssl-dev \
    ca-certificates \
    libasound2-dev \
    wget \
    pkg-config \
    autoconf \
    automake \
    curl \
    gcc \
    g++ \
    clang \
    protobuf-compiler \
    libprotobuf-dev \
    cmake

ENV PROTOC=/usr/bin/protoc

# Download and install OpenSSL
RUN wget -O - https://www.openssl.org/source/openssl-1.1.1u.tar.gz | tar zxf - \
    && cd openssl-1.1.1u \
    && ./config --prefix=/usr/local \
    && make -j $(nproc) \
    && make install_sw install_ssldirs

# Download and extract Microsoft Cognitive Services Speech SDK
RUN wget -O SpeechSDK-Linux-1.22.0.tar.gz "https://github.com/jabber-tools/cognitive-services-speech-sdk-rs-files/blob/main/SpeechSDK/1.22.0/linux/SpeechSDK-Linux-1.22.0.tar.gz?raw=true" && \
    tar -xzf SpeechSDK-Linux-1.22.0.tar.gz && \
    cp SpeechSDK-Linux-1.22.0/lib/x64/*.so /usr/local/lib/ && \
    ldconfig

# Update library links and cache
RUN ldconfig -v

# Set environment variables
ENV MS_COG_SVC_SPEECH_SKIP_BINDGEN=1
ENV SSL_CERT_DIR=/etc/ssl/certs
ENV LD_LIBRARY_PATH=/usr/local/lib:$LD_LIBRARY_PATH
ENV LIBCLANG_PATH=/usr/lib/llvm-10/lib

# Set the working directory
WORKDIR /usr/src/rust-phonebot

# Copy current directory contents into the container
COPY . .

# Install the application
RUN cargo install --path .

# Run the application
CMD ["rust-phonebot"]
