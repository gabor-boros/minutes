package utils_test

import (
	"regexp"
	"testing"

	"github.com/gabor-boros/minutes/internal/pkg/utils"
	"github.com/stretchr/testify/require"
)

func TestIsRegexSet(t *testing.T) {
	re := regexp.MustCompile("^$")
	require.True(t, utils.IsRegexSet(re))
}

func TestIsRegexSet_EmptyString(t *testing.T) {
	re := regexp.MustCompile("")
	require.False(t, utils.IsRegexSet(re))
}

func TestIsRegexSet_NilPointer(t *testing.T) {
	require.False(t, utils.IsRegexSet(nil))
}
