package util

import (
	"encoding/xml"
	"fmt"
)

func NameToKey(input xml.Name) (result string) {
	if input.Space == "" {
		result = input.Local
	} else {
		result = fmt.Sprintf("%s:%s", input.Space, input.Local)
	}
	return
}
