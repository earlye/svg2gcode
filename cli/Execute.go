// -*- tab-width: 2 -*-
/// coverage-ignore
package cli

import (
	goErrors "github.com/go-errors/errors"
	"log"
)

func Execute() int {
	if err := rootCmd.Execute(); err != nil {
		var goErr *goErrors.Error
		goErrors.As(err, &goErr)
		if goErr != nil {
			log.Printf("[DEBUG] %v\n", goErr.ErrorStack())
		}
		return 1
	}
	return 0
}
