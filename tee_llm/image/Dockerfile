FROM alpine:latest
# FROM debian:stable-slim

# Install necessary libraries and tools
RUN apk update && apk add --no-cache build-base git

# Clone the repository and build the llama-cli
RUN git clone https://github.com/ggerganov/llama.cpp.git && \
    cd llama.cpp && \
    make && \
    cp llama-cli /llama-cli && \
    cd .. && \
    rm -rf llama.cpp

COPY models/  .

# COPY llama-cli models/  .

CMD /llama-cli -m /llama-2-7b-chat.Q4_0.gguf -p "How to combine AI and blockchain?"  -n 800  --temp 0