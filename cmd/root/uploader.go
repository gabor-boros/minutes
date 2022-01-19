package root

import (
	"errors"

	"github.com/gabor-boros/minutes/internal/pkg/client"
	"github.com/gabor-boros/minutes/internal/pkg/client/tempo"
	"github.com/spf13/viper"
)

var (
	ErrNoTargetImplementation = errors.New("no target implementation found")
)

func getUploader() (client.Uploader, error) {
	switch viper.GetString("target") {
	case "tempo":
		return tempo.NewUploader(&tempo.ClientOpts{
			BaseClientOpts: client.BaseClientOpts{
				Timeout: client.DefaultRequestTimeout,
			},
			BasicAuth: client.BasicAuth{
				Username: viper.GetString("tempo-username"),
				Password: viper.GetString("tempo-password"),
			},
			BaseURL: viper.GetString("tempo-url"),
		})
	default:
		return nil, ErrNoTargetImplementation
	}
}
