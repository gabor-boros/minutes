package utils_test

import (
	"testing"
	"time"

	"github.com/gabor-boros/minutes/internal/cmd/utils"

	"github.com/stretchr/testify/require"
)

func TestTruncate(t *testing.T) {
	var truncated string
	text := "This is a short text"

	truncated = utils.Truncate(text, len(text))
	require.Equal(t, text, truncated)

	truncated = utils.Truncate(text, 200)
	require.Equal(t, text, truncated)

	truncated = utils.Truncate(text, 0)
	require.Equal(t, text, truncated)

	truncated = utils.Truncate(text, -1)
	require.Equal(t, text, truncated)

	truncated = utils.Truncate(text, 4)
	require.Equal(t, "T...", truncated)

	truncated = utils.Truncate(text, 18)
	require.Equal(t, "This is a short...", truncated)
}

func TestIsSliceContains(t *testing.T) {
	require.False(t, utils.IsSliceContains("test", []string{}))
	require.True(t, utils.IsSliceContains("test", []string{"test"}))
	require.False(t, utils.IsSliceContains("test", []string{"testing"}))
	require.True(t, utils.IsSliceContains("test", []string{"testing", "test"}))
}

func TestGetTime(t *testing.T) {
	var parsed time.Time
	var err error

	year, month, day := time.Now().Date()

	parsed, err = utils.GetTime("2021-01-01 01:00:00", "2006-01-02 15:04:05")
	require.Nil(t, err)
	require.Equal(t, time.Date(2021, 1, 1, 1, 0, 0, 0, time.Local), parsed)

	parsed, err = utils.GetTime("", "2006-01-02")
	require.Nil(t, err)
	require.Equal(t, time.Date(year, month, day, 0, 0, 0, 0, time.Local), parsed)
}
