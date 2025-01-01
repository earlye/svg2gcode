package logx

import (
	"fmt"
	"io"
	_ "log"
	"os"

	"runtime"
	"svg2gcode/util"
	"slices"
)

type Indenter struct {
	Depth int
	// The underlying io.Writer where log messages will be sent after indentation is added.
	Output io.Writer
}

var stdIndenter = &Indenter{
	Output: os.Stderr,
}

func Default() *Indenter{
	return stdIndenter
}

func (this* Indenter) SetOutput(output io.Writer) {
	this.Output = output
}

func caller(depth int) (result string) {
	pc, _, _, _ := runtime.Caller(depth)
	details := runtime.FuncForPC(pc)
	result = details.Name()
	return
}


func (this* Indenter) AdjustDepth(amount int) {
	this.Depth = this.Depth + amount
	// log.Printf("[DEBUG] new depth: %d [%s]\n", this.Depth, caller(4))
}

func (this* Indenter) Indent(amount int) (result func()){
	result = func() {
		this.AdjustDepth(-amount)
	}
	this.AdjustDepth(amount)
	return
}

func Indent(amount int) (result func()){
	result = stdIndenter.Indent(amount)
	return
}


func SetOutput(output io.Writer) {
	stdIndenter.SetOutput(output)
}

func (this *Indenter) WriteChunk(p []byte, result *int) (err error) {
	n, err := this.Output.Write(p)
	*result = *result + n
	return
}

func (this *Indenter) WriteIndentedChunk(offset int, p []byte, result *int) (err error) {
	currentOffset := 0
	start := 0
	for {
		pos := util.Index(p, '\n', start)
		// this.WriteChunk([]byte(fmt.Sprintf("start: %d pos: %d", start, pos)), result)
		if (pos != -1) {
			if currentOffset != 0 {
				this.WriteChunk([]byte(fmt.Sprintf("%*s|%*s   ", currentOffset, "", this.Depth, "")), result)
			}
			this.WriteChunk(p[start:pos+1], result)
			currentOffset = offset
			start = pos + 1
		} else {
			this.WriteChunk(p[start:], result)
			return
		}
	}
}

func (this *Indenter) Write(p []byte) (result int, err error) {
	pos := slices.Index(p, ']')
	if (pos != -1) {
		err = this.WriteChunk(p[0:pos+1], &result)
		err = this.WriteChunk([]byte(fmt.Sprintf("|%*s", this.Depth, "")), &result)
		err = this.WriteIndentedChunk(pos+1, p[pos+1:], &result)
		return
	} else {
		result, err = this.Output.Write(p)
		return
	}
}
