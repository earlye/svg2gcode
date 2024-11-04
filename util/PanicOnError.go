package util

func PanicOnError(err error) {
	if err != nil {
		panic("Unexpected error")
	}
}
