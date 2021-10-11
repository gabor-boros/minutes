package cmd

import (
	"context"
	"errors"
	"fmt"
	"net/http"
	"net/url"
	"os"
	"regexp"
	"strings"
	"time"

	"github.com/gabor-boros/minutes/internal/cmd/printer"
	"github.com/gabor-boros/minutes/internal/cmd/utils"
	"github.com/gabor-boros/minutes/internal/pkg/client/clockify"
	"github.com/gabor-boros/minutes/internal/pkg/client/tempo"

	"github.com/jedib0t/go-pretty/v6/table"

	"github.com/gabor-boros/minutes/internal/pkg/client"
	"github.com/gabor-boros/minutes/internal/pkg/worklog"
	"github.com/spf13/cobra"
	"github.com/spf13/viper"
)

const (
	program           string = "minutes"
	defaultDateFormat string = "2006-01-02 15:04:05"
)

var (
	configFile string
	envPrefix  string

	version string
	commit  string
	date    string

	sources = []string{"clockify", "tempo"}
	targets = []string{"tempo"}

	ErrNoSourceImplementation = errors.New("no source implementation found")
	ErrNoTargetImplementation = errors.New("no target implementation found")

	rootCmd = &cobra.Command{
		Use:   program,
		Short: "Sync worklogs between multiple time trackers, invoicing, and bookkeeping software.",
		Long: `
Minutes is a CLI tool for synchronizing work logs between multiple time
trackers, invoicing, and bookkeeping software to make entrepreneurs'
daily work easier.

Every source and destination comes with their specific flags. Before using any
flags, check the related documentation.

Minutes comes with absolutely NO WARRANTY; for more information, visit the
project's home page.

Project home page: https://github.com/gabor-boros/minutes
Report bugs at: https://github.com/gabor-boros/minutes/issues
Report security issues to: gabor.brs@gmail.com`,
		Run: runRootCmd,
	}
)

func init() {
	envPrefix = strings.ToUpper(program)

	cobra.OnInitialize(initConfig)

	initCommonFlags()
	initClockifyFlags()
	initTempoFlags()
}

func initConfig() {
	if configFile != "" {
		viper.SetConfigName(configFile)
	} else {
		homeDir, err := os.UserHomeDir()
		cobra.CheckErr(err)

		configDir, err := os.UserConfigDir()
		cobra.CheckErr(err)

		viper.AddConfigPath(homeDir)
		viper.AddConfigPath(configDir)
		viper.SetConfigName("." + program)
		viper.SetConfigType("toml")
	}

	viper.SetEnvPrefix(envPrefix)
	viper.AutomaticEnv()

	if err := viper.ReadInConfig(); err == nil {
		fmt.Println("Using config file:", viper.ConfigFileUsed(), configFile)
	} else {
		cobra.CheckErr(err)
	}

	// Bind flags to config value
	cobra.CheckErr(viper.BindPFlags(rootCmd.Flags()))
}

func initCommonFlags() {
	rootCmd.PersistentFlags().StringVar(&configFile, "config", "", fmt.Sprintf("config file (default is $HOME/.%s.yaml)", program))

	rootCmd.Flags().StringP("start", "", "", "set the start date (defaults to 00:00:00)")
	rootCmd.Flags().StringP("end", "", "", "set the end date (defaults to now)")
	rootCmd.Flags().StringP("date-format", "", defaultDateFormat, "set start and end date format (in Go style)")

	rootCmd.Flags().StringP("source-user", "", "", "set the source user ID")
	rootCmd.Flags().StringP("source", "s", "", fmt.Sprintf("set the source of the sync %v", sources))

	rootCmd.Flags().StringP("target-user", "", "", "set the source user ID")
	rootCmd.Flags().StringP("target", "t", "", fmt.Sprintf("set the target of the sync %v", targets))

	rootCmd.Flags().StringSliceP("table-sort-by", "", []string{printer.ColumnStart, printer.ColumnProject, printer.ColumnTask, printer.ColumnSummary}, fmt.Sprintf("sort table by column %v", printer.Columns))
	rootCmd.Flags().StringSliceP("table-hide-column", "", []string{}, fmt.Sprintf("hide table column %v", printer.HideableColumns))

	rootCmd.Flags().BoolP("tasks-as-tags", "", false, "treat tags matching the value of tasks-as-tags-regex as tasks")
	rootCmd.Flags().StringP("tasks-as-tags-regex", "", "", "regex of the task pattern")

	rootCmd.Flags().BoolP("round-to-closest-minute", "", false, "round time to closest minute")
	rootCmd.Flags().BoolP("force-billed-duration", "", false, "treat every second spent as billed")

	rootCmd.Flags().BoolP("dry-run", "", false, "fetch entries, but do not sync them")
	rootCmd.Flags().BoolP("verbose", "", false, "print verbose messages")
	rootCmd.Flags().BoolP("version", "", false, "show command version")
}

func initClockifyFlags() {
	rootCmd.Flags().StringP("clockify-url", "", "", "set the base URL")
	rootCmd.Flags().StringP("clockify-api-key", "", "", "set the API key")
	rootCmd.Flags().StringP("clockify-workspace", "", "", "set the workspace ID")
}

func initTempoFlags() {
	rootCmd.Flags().StringP("tempo-url", "", "", "set the base URL")
	rootCmd.Flags().StringP("tempo-username", "", "", "set the login user ID")
	rootCmd.Flags().StringP("tempo-password", "", "", "set the login password")
}

func validateFlags() {
	source := viper.GetString("source")
	target := viper.GetString("target")

	if source == target {
		cobra.CheckErr("sync source cannot match the target")
	}

	if !utils.IsSliceContains(source, sources) {
		cobra.CheckErr(fmt.Sprintf("\"%s\" is not part of the supported sources %v\n", source, sources))
	}

	if !utils.IsSliceContains(target, targets) {
		cobra.CheckErr(fmt.Sprintf("\"%s\" is not part of the supported targets %v\n", target, targets))
	}

	if viper.GetBool("tasks-as-tags") {
		tasksAsTagsRegex := viper.GetString("tasks-as-tags-regex")

		if tasksAsTagsRegex == "" {
			cobra.CheckErr("tasks-as-tags-regex cannot be empty if tasks-as-tags is set")
		}

		_, err := regexp.Compile(tasksAsTagsRegex)
		cobra.CheckErr(err)
	}

	for _, sortBy := range viper.GetStringSlice("table-sort-by") {
		column := sortBy

		if strings.HasPrefix(column, "-") {
			column = sortBy[1:]
		}

		if !utils.IsSliceContains(column, printer.Columns) {
			cobra.CheckErr(fmt.Sprintf("\"%s\" is not part of the sortable columns %v\n", column, printer.Columns))
		}
	}

	for _, column := range viper.GetStringSlice("table-hide-column") {
		if !utils.IsSliceContains(column, printer.HideableColumns) {
			cobra.CheckErr(fmt.Sprintf("\"%s\" is not part of the hideable columns %v\n", column, printer.HideableColumns))
		}
	}
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

func runRootCmd(_ *cobra.Command, _ []string) {
	var err error

	if viper.GetBool("version") {
		fmt.Printf("%s version %s, commit %s (%s)\n", program, version, commit[:8], date)
		os.Exit(0)
	}

	validateFlags()

	dateFormat := viper.GetString("date-format")

	start, err := utils.GetTime(viper.GetString("start"), dateFormat)
	cobra.CheckErr(err)

	rawEnd := viper.GetString("end")
	end, err := utils.GetTime(rawEnd, dateFormat)
	cobra.CheckErr(err)

	// No end date was set, hence we are setting the end date to next day midnight
	if rawEnd == "" {
		end = end.Add(time.Hour * 24)
	}

	fetcher, err := getFetcher()
	cobra.CheckErr(err)

	uploader, err := getUploader()
	cobra.CheckErr(err)

	entries, err := fetcher.FetchEntries(context.Background(), &client.FetchOpts{
		End:   end,
		Start: start,
		User:  viper.GetString("source-user"),
	})
	cobra.CheckErr(err)

	wl := worklog.NewWorklog(entries)
	completeEntries := wl.CompleteEntries()
	incompleteEntries := wl.IncompleteEntries()

	columnTruncates := map[string]int{}
	err = viper.UnmarshalKey("table-column-truncates", &columnTruncates)
	cobra.CheckErr(err)

	tablePrinter := printer.NewTablePrinter(&printer.TablePrinterOpts{
		BasePrinterOpts: printer.BasePrinterOpts{
			Output:        os.Stdout,
			AutoIndex:     true,
			Title:         fmt.Sprintf("Worklog entries (%s - %s)", start.Local().String(), end.Local().String()),
			SortBy:        viper.GetStringSlice("table-sort-by"),
			HiddenColumns: viper.GetStringSlice("table-hide-column"),
		},
		Style: table.StyleLight,
		ColumnConfig: printer.ParseColumnConfigs(
			"table-column-config.%s",
			viper.GetStringSlice("table-hide-column"),
		),
		ColumnTruncates: columnTruncates,
	})

	err = tablePrinter.Print(completeEntries, incompleteEntries)
	cobra.CheckErr(err)

	if strings.ToLower(utils.Prompt("Continue? [y/n]: ")) != "y" {
		fmt.Println("User interruption. Aborting.")
		os.Exit(0)
	}

	if !viper.GetBool("dry-run") {
		err = uploader.UploadEntries(context.Background(), completeEntries, &client.UploadOpts{
			RoundToClosestMinute:   viper.GetBool("round-to-closest-minute"),
			TreatDurationAsBilled:  viper.GetBool("force-billed-duration"),
			CreateMissingResources: false,
			User:                   viper.GetString("target-user"),
		})
		cobra.CheckErr(err)
	}
}

func Execute(buildVersion string, buildCommit string, buildDate string) {
	version = buildVersion
	commit = buildCommit
	date = buildDate

	cobra.CheckErr(rootCmd.Execute())
}
