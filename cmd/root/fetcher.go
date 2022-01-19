package root

import (
	"errors"
	"os/exec"

	"github.com/gabor-boros/minutes/internal/pkg/client"
	"github.com/gabor-boros/minutes/internal/pkg/client/clockify"
	"github.com/gabor-boros/minutes/internal/pkg/client/harvest"
	"github.com/gabor-boros/minutes/internal/pkg/client/tempo"
	"github.com/gabor-boros/minutes/internal/pkg/client/timewarrior"
	"github.com/gabor-boros/minutes/internal/pkg/client/toggl"
	"github.com/spf13/viper"
)

var (
	ErrNoSourceImplementation = errors.New("no source implementation found")
)

func getClockifyFetcher() (client.Fetcher, error) {
	return clockify.NewFetcher(&clockify.ClientOpts{
		BaseClientOpts: client.BaseClientOpts{
			Timeout: client.DefaultRequestTimeout,
		},
		TokenAuth: client.TokenAuth{
			Header: "X-Api-Key",
			Token:  viper.GetString("clockify-api-key"),
		},
		BaseURL:   viper.GetString("clockify-url"),
		Workspace: viper.GetString("clockify-workspace"),
	})
}

func getHarvestFetcher() (client.Fetcher, error) {
	return harvest.NewFetcher(&harvest.ClientOpts{
		BaseClientOpts: client.BaseClientOpts{
			Timeout: client.DefaultRequestTimeout,
		},
		TokenAuth: client.TokenAuth{
			TokenName: "Bearer",
			Token:     viper.GetString("harvest-api-key"),
		},
		BaseURL: "https://api.harvestapp.com",
		Account: viper.GetInt("harvest-account"),
	})
}

func getTempoFetcher() (client.Fetcher, error) {
	return tempo.NewFetcher(&tempo.ClientOpts{
		BaseClientOpts: client.BaseClientOpts{
			Timeout: client.DefaultRequestTimeout,
		},
		BasicAuth: client.BasicAuth{
			Username: viper.GetString("tempo-username"),
			Password: viper.GetString("tempo-password"),
		},
		BaseURL: viper.GetString("tempo-url"),
	})
}

func getTimeWarriorFetcher() (client.Fetcher, error) {
	return timewarrior.NewFetcher(&timewarrior.ClientOpts{
		BaseClientOpts: client.BaseClientOpts{
			Timeout: client.DefaultRequestTimeout,
		},
		CLIClient: client.CLIClient{
			Command:            viper.GetString("timewarrior-command"),
			CommandArguments:   viper.GetStringSlice("timewarrior-arguments"),
			CommandCtxExecutor: exec.CommandContext,
		},
		UnbillableTag:   viper.GetString("timewarrior-unbillable-tag"),
		ClientTagRegex:  viper.GetString("timewarrior-client-tag-regex"),
		ProjectTagRegex: viper.GetString("timewarrior-project-tag-regex"),
	})
}

func getTogglFetcher() (client.Fetcher, error) {
	return toggl.NewFetcher(&toggl.ClientOpts{
		BaseClientOpts: client.BaseClientOpts{
			Timeout: client.DefaultRequestTimeout,
		},
		BasicAuth: client.BasicAuth{
			Username: viper.GetString("toggl-api-key"),
			Password: "api_token",
		},
		BaseURL:   "https://api.track.toggl.com",
		Workspace: viper.GetInt("toggl-workspace"),
	})
}

func getFetcher() (client.Fetcher, error) {

	var fetcher client.Fetcher
	var err error

	switch viper.GetString("source") {
	case "clockify":
		fetcher, err = getClockifyFetcher()
	case "harvest":
		fetcher, err = getHarvestFetcher()
	case "tempo":
		fetcher, err = getTempoFetcher()
	case "timewarrior":
		fetcher, err = getTimeWarriorFetcher()
	case "toggl":
		fetcher, err = getTogglFetcher()
	default:
		fetcher, err = nil, ErrNoSourceImplementation
	}

	return fetcher, err
}
