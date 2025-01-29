MODULE=$(shell grep ^module go.mod | awk '{print $$2}')
GOFILES=$(shell find . -type f -name "*.go")
SEMVER=0.0.1
VERTAG=-$(shell git rev-parse --short HEAD || echo "0000000")
PATH := $(PATH):$(HOME)/go/bin
PACKAGE_DIRS:=$(filter-out ./, $(sort $(dir $(GOFILES))))
DEPENDENCY_FILES:=$(patsubst %,%.dependencies,$(PACKAGE_DIRS))

$(info Entering directory `$(shell pwd)') # '`

all: bin/svg2gcode

.PHONY: test
test: 
	rm -f .coverage.*
	$(MAKE) .coverage.html

.PHONY: dependencies
dependencies:
	rm -f .dependencies $(DEPENDENCY_FILES)
	$(MAKE) .dependencies

.dependencies: go.mod $(DEPENDENCY_FILES)
	echo "Getting dependencies"
	go get ${MODULE}
	go get -t ${MODULE}
	touch $@

%/.dependencies:
	go get ${MODULE}/$(dir $@)
	go get -t ${MODULE}/$(dir $@)
	touch $@

.uncover:
	go install github.com/gregoryv/uncover/cmd/uncover@b56b18c8233ef7a454eed0c045ecf19b30da36dd
	touch $@

bin/svg2gcode: test $(GOFILES) go.mod Makefile
	GOOS=darwin && \
	go version && \
	go build -buildvcs=false -ldflags="-X 'svg2gcode/consts.Version=$(SEMVER)$(VERTAG)'" -o "$(abspath $@)"
	test -f "$(abspath $@)"
	chmod +x "$(abspath $@)"

examples/%.gcode : examples/%.svg bin/svg2gcode
	bin/svg2gcode $< > $@
	# cat $@

.coverage.html: .coverage.out 
	go tool cover -html=.coverage.out -o $(abspath $@)

.coverage.out: .coverage.out.tmp .uncover
	./munge-coverage.sh
	$(HOME)/go/bin/uncover -min 5.0 .coverage.out | ansifilter -B -k | sed "s/\\[color=#00cd00\\]//g;s#\\[/color\\]##g"

.coverage.out.tmp: $(GOFILES)
	go test -v ./... -count=1 -coverprofile=$(notdir $@) | sed "s/^coverage:/coverage :/g"

testParseSvgPathData: $(GOFILES)
	go clean -testcache
	go test -v -count=1 -run ParseSvgPathData svg2gcode/svg
