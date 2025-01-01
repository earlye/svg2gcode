package util

func Index[S ~[]E, E comparable](s S, v E, start int) int {
	for i := start; i < len(s); i = i + 1 {
		if s[i] == v {
			return i
		}
	}
	return -1
}
