package cmd

import (
	"bufio"
	"errors"
	"fmt"
	"github.com/gabor-boros/minutes/internal/pkg/client"
	"github.com/gabor-boros/minutes/internal/pkg/client/clockify"
	"github.com/gabor-boros/minutes/internal/pkg/client/tempo"
	"github.com/gabor-boros/minutes/internal/pkg/worklog"
	"github.com/spf13/cobra"
	"github.com/spf13/viper"
	"net/http"
	"net/url"
	"os"
	"strings"
	"text/tabwriter"
	"time"
)

var (
	ErrNoSourceImplementation = errors.New("no source implementation found")
	ErrNoTargetImplementation = errors.New("no target implementation found")
)

func printEntries(header string, entries []worklog.Entry) {
	w := new(tabwriter.Writer)
	w.Init(os.Stdout, 8, 8, 0, '\t', 0)

	fmt.Printf("%s:\n", header)

	if len(entries) == 0 {
		fmt.Printf("No items found.\n\n")
		return
	}

	_, _ = fmt.Fprintf(w, worklogTableFormat, "Task", "Summary", "Billed", "Unbilled")
	_, _ = fmt.Fprintf(w, worklogTableFormat, "", "", "", "")

	totalBilledDuration := time.Duration(0)
	totalUnbilledDuration := time.Duration(0)

	for _, entry := range entries {
		_, _ = fmt.Fprintf(w, worklogTableFormat, entry.Task.Name, entry.Summary, entry.BillableDuration.String(), entry.UnbillableDuration.String())
		totalBilledDuration += entry.BillableDuration
		totalUnbilledDuration += entry.UnbillableDuration
	}

	_, _ = fmt.Fprintf(w, worklogTableFormat, "", "", "", "")
	_, _ = fmt.Fprintf(w, worklogTableFormat, "", "Total time spent:", totalBilledDuration.String(), totalUnbilledDuration.String())
	_ = w.Flush()

	// Do an empty print to break lines
	fmt.Println()
}

func prompt(message string) string {
	fmt.Printf("%s: ", message)

	reader := bufio.NewReader(os.Stdin)
	input, err := reader.ReadString('\n')
	cobra.CheckErr(err)

	return strings.TrimSpace(input)
}

func isSliceContains(item string, slice []string) bool {
	for _, s := range slice {
		if s == item {
			return true
		}
	}

	return false
}

func getTime(rawDate string) (time.Time, error) {
	var date time.Time
	var err error

	if rawDate == "" {
		year, month, day := time.Now().Date()
		date = time.Date(year, month, day, 0, 0, 0, 0, time.Local)
	} else {
		return time.Parse(viper.GetString("date-format"), rawDate)
	}

	return date, err
}

func getClientOpts(urlFlag string, usernameFlag string, passwordFlag string, tokenFlag string, tokenHeader string) (*client.BaseClientOpts, error) {
	opts := &client.BaseClientOpts{
		HTTPClientOptions: client.HTTPClientOptions{
			HTTPClient:  http.DefaultClient,
			TokenHeader: tokenHeader,
		},
		TasksAsTags:      viper.GetBool("tasks-as-tags"),
		TasksAsTagsRegex: viper.GetString("tasks-as-tags-regex"),
	}

	baseURL, err := url.Parse(viper.GetString(urlFlag))
	if err != nil {
		return opts, err
	}

	if usernameFlag != "" {
		opts.Username = viper.GetString(usernameFlag)
	}

	if passwordFlag != "" {
		opts.Password = viper.GetString(passwordFlag)
	}

	if tokenFlag != "" {
		opts.Token = viper.GetString(tokenFlag)
	}

	opts.BaseURL = baseURL.String()

	return opts, nil
}

func getFetcher() (client.Fetcher, error) {
	switch viper.GetString("source") {
	case "clockify":
		opts, err := getClientOpts(
			"clockify-url",
			"",
			"",
			"clockify-api-key",
			"X-Api-Key",
		)

		if err != nil {
			return nil, err
		}

		return clockify.NewClient(&clockify.ClientOpts{
			BaseClientOpts: *opts,
			Workspace:      viper.GetString("clockify-workspace"),
		}), nil
	case "tempo":
		opts, err := getClientOpts(
			"tempo-url",
			"tempo-username",
			"tempo-password",
			"",
			"",
		)

		if err != nil {
			return nil, err
		}

		return tempo.NewClient(&tempo.ClientOpts{
			BaseClientOpts: *opts,
		}), nil
	default:
		return nil, ErrNoSourceImplementation
	}
}

func getUploader() (client.Uploader, error) {
	switch viper.GetString("target") {
	case "tempo":
		opts, err := getClientOpts(
			"tempo-url",
			"tempo-username",
			"tempo-password",
			"",
			"",
		)

		if err != nil {
			return nil, err
		}

		return tempo.NewClient(&tempo.ClientOpts{
			BaseClientOpts: *opts,
		}), nil
	default:
		return nil, ErrNoTargetImplementation
	}
}
