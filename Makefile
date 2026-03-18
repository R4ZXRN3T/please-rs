.PHONY: build clean install

ifeq ($(OS),Windows_NT)
UNAME_S := Windows
else
UNAME_S := $(shell uname -s)
endif

build:
ifeq ($(UNAME_S),Linux)
	@bash scripts/build-linux.sh
else ifeq ($(UNAME_S),Darwin)
	@bash scripts/build-macos.sh
else ifeq ($(OS),Windows_NT)
	@pwsh -ExecutionPolicy Bypass -File scripts/build-windows.ps1
else
	@echo "Unsupported operating system"
	@exit 1
endif

install:
ifeq ($(UNAME_S),Linux)
	$(MAKE) build
	cp ./final/*/please /usr/bin
	$(MAKE) clean
else ifeq ($(UNAME_S),Darwin)
	$(MAKE) build
	cp ./final/*/please /usr/local/bin
	$(MAKE) clean
else ifeq ($(OS),Windows_NT)
	$(MAKE) build
	@pwsh -NoProfile -ExecutionPolicy Bypass -File scripts/install-windows.ps1
	$(MAKE) clean
else
	@echo "Unsupported operating system"
	@exit 1
endif

clean:
ifeq ($(UNAME_S),Linux)
	rm -rf ./target
else ifeq ($(UNAME_S),Darwin)
	rm -rf ./target
else ifeq ($(OS),Windows_NT)
	rmdir /s /q .\target
else
	@echo "Unsupported operating system"
	@exit 1
endif

