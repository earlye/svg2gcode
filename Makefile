MODULE=$(shell grep ^module go.mod | awk '{print $$2}')
GOFILES=$(shell find . -type f -name "*.go")
SEMVER=0.0.1
VERTAG=-$(shell git rev-parse --short HEAD || echo "0000000")

all: bin/svg2gcode

.dependencies: go.mod $(NETRC)
	echo "Getting dependencies"
	go get ${MODULE}
	go get -t ${MODULE}
	go install github.com/gregoryv/uncover/cmd/uncover@b56b18c8233ef7a454eed0c045ecf19b30da36dd
	touch $@

bin/svg2gcode: $(GOFILES) go.mod Makefile
	GOOS=darwin && \
	go version && \
	go build -buildvcs=false -ldflags="-X 'svg2gcode/cmd.Version=$(SEMVER)$(VERTAG)'" -o "$(abspath $@)"
	test -f "$(abspath $@)"
	chmod +x "$(abspath $@)"

run: bin/svg2gcode
	bin/svg2gcode --help
