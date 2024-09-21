FROM debian:12
ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get update
RUN apt-get install -y locales locales-all
ENV LANG=en_US.UTF-8
ENV LC_ALL=en_US.UTF-8
ENV LC_CTYPE=en_US.UTF-8
ENV LC_ALL=en_US.UTF-8
ENV LANGUAGE=en_US.UTF-8

RUN echo "LC_ALL=en_US.UTF-8" >> /etc/environment
RUN apt-get update
RUN apt-get upgrade -y
RUN apt-get install -y curl tmux nano net-tools procps htop git build-essential cmake pkg-config libssl-dev

RUN apt-get update && apt-get install -y apache2 elinks
COPY zfs-unlocker.conf /etc/apache2/sites-available/

RUN useradd -m runner

USER runner

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y

RUN cd /home/runner/
WORKDIR /home/runner/

ENV PATH="/home/runner/.cargo/bin:$PATH"
RUN echo "export /home/runner/.cargo/bin:$PATH" > /home/runner/.bashrc

RUN rustup target add wasm32-wasi
RUN rustup target add wasm32-unknown-unknown
RUN cargo install trunk

RUN cd /home/runner/
WORKDIR /home/runner/

# Build the website
RUN git clone https://github.com/TheQuantumPhysicist/sam-remote-zfs-unlocker
RUN  cd /home/runner/sam-remote-zfs-unlocker/frontend
WORKDIR /home/runner/sam-remote-zfs-unlocker/frontend
RUN cargo update
RUN trunk build --release

RUN mkdir /home/runner/app/
RUN cp -r /home/runner/sam-remote-zfs-unlocker/frontend/dist /home/runner/app/
RUN cd /home/runner/app/
WORKDIR /home/runner/app/
RUN mv dist webpage

# Clean up the source and build dirs
RUN rm -rf /home/runner/sam-remote-zfs-unlocker/

USER root

RUN chown -R www-data:www-data /home/runner/app/webpage/

RUN a2ensite zfs-unlocker
RUN a2dissite 000-default

CMD ["/bin/sh", "-c", "apt-get update && apt-get upgrade -y && /usr/sbin/apache2ctl -DFOREGROUND"]
