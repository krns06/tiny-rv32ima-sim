FROM ubuntu:latest
RUN apt-get update &&\
    apt-get install -y autoconf automake autotools-dev curl python3 python3-pip python3-tomli libmpc-dev libmpfr-dev libgmp-dev gawk build-essential bison flex texinfo gperf libtool patchutils bc zlib1g-dev libexpat-dev ninja-build git cmake libglib2.0-dev libslirp-dev &&\
     apt-get clean &&\
    rm -rf /var/lib/apt/lists/* &&\
    git clone https://github.com/riscv/riscv-gnu-toolchain &&\
    git clone https://github.com/riscv-software-src/riscv-tests.git &&\
    mkdir tests
WORKDIR riscv-gnu-toolchain
RUN  git switch -d 2026.06.06 &&\
     sed -i '/shallow = true/d' .gitmodules &&\
     sed -i 's/--depth 1//g' Makefile.in &&\
    ./configure --prefix=/opt/riscv &&\
    make -j6
WORKDIR /riscv-tests
RUN git submodule update --init --recursive &&\
    autoconf &&\
    ./configure --prefix=/tests &&\
    PATH=/opt/riscv/bin:$PATH make -j6 && make install
WORKDIR /tests/share/riscv-tests/isa
RUN mkdir /flats &&\
    ls | grep -v \.dump$ | fgrep -v Makefile | xargs -INAME /opt/riscv/bin/riscv64-unknown-elf-objcopy -O binary NAME /flats/NAME.bin &&\
    ls | grep \.dump$ | xargs -INAME mv NAME /flats/NAME

WORKDIR /
RUN tar cvf isa.tar.xz /flats
