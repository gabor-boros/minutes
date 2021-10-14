package utils_test

import (
	"reflect"
	"testing"
	"time"

	"github.com/gabor-boros/minutes/internal/cmd/utils"
	"github.com/stretchr/testify/require"
)

func TestNewProgressWriter(t *testing.T) {
	progressWriter := utils.NewProgressWriter(time.Millisecond * 100)
	require.Equal(t, "*progress.Progress", reflect.TypeOf(progressWriter).String())
}
