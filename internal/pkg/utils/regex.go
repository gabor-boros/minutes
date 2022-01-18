package utils

import (
	"regexp"
)

// IsRegexSet returns true if the regex is not nil nor an empty string,
// otherwise, it returns false.
func IsRegexSet(r *regexp.Regexp) bool {
	return r != nil && r.String() != ""
}
