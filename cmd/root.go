package cmd

import (
	"context"
	"errors"
	"fmt"
	"os"
	"os/exec"
	"regexp"
	"strings"
	"time"

	"github.com/gabor-boros/minutes/internal/pkg/client/timewarrior"

	"github.com/gabor-boros/minutes/internal/pkg/client/clockify"

	"github.com/gabor-boros/minutes/internal/pkg/client/toggl"

	"github.com/gabor-boros/minutes/internal/cmd/utils"
	"github.com/gabor-boros/minutes/internal/pkg/client/tempo"

	"github.com/jedib0t/go-pretty/v6/progress"
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

	sources = []string{"clockify", "tempo", "timewarrior", "toggl"}
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

Project home page: https://gabor-boros.github.io/minutes
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
	initTimewarriorFlags()
	initTogglFlags()
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

	rootCmd.Flags().StringSliceP("table-sort-by", "", []string{utils.ColumnStart, utils.ColumnProject, utils.ColumnTask, utils.ColumnSummary}, fmt.Sprintf("sort table by column %v", utils.Columns))
	rootCmd.Flags().StringSliceP("table-hide-column", "", []string{}, fmt.Sprintf("hide table column %v", utils.HideableColumns))

	rootCmd.Flags().BoolP("tags-as-tasks", "", false, "treat tags matching the value of tags-as-tasks-regex as tasks")
	rootCmd.Flags().StringP("tags-as-tasks-regex", "", "", "regex of the task pattern")

	rootCmd.Flags().BoolP("round-to-closest-minute", "", false, "round time to closest minute")
	rootCmd.Flags().BoolP("force-billed-duration", "", false, "treat every second spent as billed")

	rootCmd.Flags().StringP("filter-client", "", "", "filter for client name after fetching")
	rootCmd.Flags().StringP("filter-project", "", "", "filter for project name after fetching")

	rootCmd.Flags().BoolP("dry-run", "", false, "fetch entries, but do not sync them")
	rootCmd.Flags().BoolP("version", "", false, "show command version")
}

func initClockifyFlags() {
	rootCmd.Flags().StringP("clockify-url", "", "https://api.clockify.me", "set the base URL")
	rootCmd.Flags().StringP("clockify-api-key", "", "", "set the API key")
	rootCmd.Flags().StringP("clockify-workspace", "", "", "set the workspace ID")
}

func initTempoFlags() {
	rootCmd.Flags().StringP("tempo-url", "", "", "set the base URL")
	rootCmd.Flags().StringP("tempo-username", "", "", "set the login user ID")
	rootCmd.Flags().StringP("tempo-password", "", "", "set the login password")
}

func initTimewarriorFlags() {
	rootCmd.Flags().StringP("timewarrior-command", "", "timew", "set the executable name")
	rootCmd.Flags().StringSliceP("timewarrior-arguments", "", []string{}, "set additional arguments")

	rootCmd.Flags().StringP("timewarrior-unbillable-tag", "", "unbillable", "set the unbillable tag")
	rootCmd.Flags().StringP("timewarrior-client-tag-regex", "", "", "regex of client tag pattern")
	rootCmd.Flags().StringP("timewarrior-project-tag-regex", "", "", "regex of project tag pattern")
}

func initTogglFlags() {
	rootCmd.Flags().StringP("toggl-api-key", "", "", "set the API key")
	rootCmd.Flags().IntP("toggl-workspace", "", 0, "set the workspace ID")
}

func validateFlags() {
	var err error
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

	if viper.GetBool("tags-as-tasks") {
		tasksAsTagsRegex := viper.GetString("tags-as-tasks-regex")

		if tasksAsTagsRegex == "" {
			cobra.CheckErr("tags-as-tasks-regex cannot be empty if tags-as-tasks is set")
		}

		_, err = regexp.Compile(tasksAsTagsRegex)
		cobra.CheckErr(err)
	}

	for _, sortBy := range viper.GetStringSlice("table-sort-by") {
		column := sortBy

		if strings.HasPrefix(column, "-") {
			column = sortBy[1:]
		}

		if !utils.IsSliceContains(column, utils.Columns) {
			cobra.CheckErr(fmt.Sprintf("\"%s\" is not part of the sortable columns %v\n", column, utils.Columns))
		}
	}

	for _, column := range viper.GetStringSlice("table-hide-column") {
		if !utils.IsSliceContains(column, utils.HideableColumns) {
			cobra.CheckErr(fmt.Sprintf("\"%s\" is not part of the hideable columns %v\n", column, utils.HideableColumns))
		}
	}

	_, err = regexp.Compile(viper.GetString("filter-client"))
	cobra.CheckErr(err)

	_, err = regexp.Compile(viper.GetString("filter-project"))
	cobra.CheckErr(err)

	switch source {
	case "timewarrior":
		if viper.GetString("timewarrior-command") == "" {
			cobra.CheckErr("timewarrior command must be set")
		}

		if viper.GetString("timewarrior-unbillable-tag") == "" {
			cobra.CheckErr("timewarrior unbillable tag must be set")
		}

		if viper.GetString("timewarrior-client-tag-regex") == "" {
			cobra.CheckErr("timewarrior client tag regex must be set")
		}

		if viper.GetString("timewarrior-project-tag-regex") == "" {
			cobra.CheckErr("timewarrior project tag regex must be set")
		}
	}
}

func getFetcher() (client.Fetcher, error) {
	switch viper.GetString("source") {
	case "clockify":
		return clockify.NewFetcher(&clockify.ClientOpts{
			BaseClientOpts: client.BaseClientOpts{
				TagsAsTasks:      viper.GetBool("tags-as-tasks"),
				TagsAsTasksRegex: viper.GetString("tags-as-tasks-regex"),
				Timeout:          0,
			},
			TokenAuth: client.TokenAuth{
				Header: "X-Api-Key",
				Token:  viper.GetString("clockify-api-key"),
			},
			BaseURL:   viper.GetString("clockify-url"),
			Workspace: viper.GetString("clockify-workspace"),
		})
	case "tempo":
		return tempo.NewFetcher(&tempo.ClientOpts{
			BaseClientOpts: client.BaseClientOpts{
				TagsAsTasks:      viper.GetBool("tags-as-tasks"),
				TagsAsTasksRegex: viper.GetString("tags-as-tasks-regex"),
				Timeout:          0,
			},
			BasicAuth: client.BasicAuth{
				Username: viper.GetString("tempo-username"),
				Password: viper.GetString("tempo-password"),
			},
			BaseURL: viper.GetString("tempo-url"),
		})
	case "timewarrior":
		return timewarrior.NewFetcher(&timewarrior.ClientOpts{
			BaseClientOpts: client.BaseClientOpts{
				TagsAsTasks:      viper.GetBool("tags-as-tasks"),
				TagsAsTasksRegex: viper.GetString("tags-as-tasks-regex"),
				Timeout:          0,
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
	case "toggl":
		return toggl.NewFetcher(&toggl.ClientOpts{
			BaseClientOpts: client.BaseClientOpts{
				TagsAsTasks:      viper.GetBool("tags-as-tasks"),
				TagsAsTasksRegex: viper.GetString("tags-as-tasks-regex"),
				Timeout:          0,
			},
			BasicAuth: client.BasicAuth{
				Username: viper.GetString("toggl-api-key"),
				Password: "api_token",
			},
			BaseURL:   "https://api.track.toggl.com",
			Workspace: viper.GetInt("toggl-workspace"),
		})
	default:
		return nil, ErrNoSourceImplementation
	}
}

func getUploader() (client.Uploader, error) {
	switch viper.GetString("target") {
	case "tempo":
		return tempo.NewUploader(&tempo.ClientOpts{
			BaseClientOpts: client.BaseClientOpts{
				TagsAsTasks:      viper.GetBool("tags-as-tasks"),
				TagsAsTasksRegex: viper.GetString("tags-as-tasks-regex"),
				Timeout:          0,
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

	// It is safe to use MustCompile when compiling regex as we already
	// validated its correctness
	wl := worklog.NewWorklog(entries, &worklog.FilterOpts{
		Client:  regexp.MustCompile(viper.GetString("filter-client")),
		Project: regexp.MustCompile(viper.GetString("filter-project")),
	})

	completeEntries := wl.CompleteEntries()
	incompleteEntries := wl.IncompleteEntries()

	columnTruncates := map[string]int{}
	err = viper.UnmarshalKey("table-column-truncates", &columnTruncates)
	cobra.CheckErr(err)

	tablePrinter := utils.NewTablePrinter(&utils.TablePrinterOpts{
		BasePrinterOpts: utils.BasePrinterOpts{
			Output:        os.Stdout,
			AutoIndex:     true,
			Title:         fmt.Sprintf("Worklog entries (%s - %s)", start.Local().String(), end.Local().String()),
			SortBy:        viper.GetStringSlice("table-sort-by"),
			HiddenColumns: viper.GetStringSlice("table-hide-column"),
		},
		Style: table.StyleLight,
		ColumnConfig: utils.ParseColumnConfigs(
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

	// In worst case, the maximum number of errors will match the number of entries
	uploadErrChan := make(chan error, len(completeEntries))

	fmt.Printf("\nUploading worklog entries:\n\n")
	if !viper.GetBool("dry-run") {
		progressUpdateFrequency := progress.DefaultUpdateFrequency
		progressWriter := utils.NewProgressWriter(progressUpdateFrequency)

		// Intentionally called as a goroutine
		go progressWriter.Render()

		uploader.UploadEntries(context.Background(), completeEntries, uploadErrChan, &client.UploadOpts{
			RoundToClosestMinute:   viper.GetBool("round-to-closest-minute"),
			TreatDurationAsBilled:  viper.GetBool("force-billed-duration"),
			CreateMissingResources: false,
			User:                   viper.GetString("target-user"),
			ProgressWriter:         progressWriter,
		})

		// Wait for at least one tracker to appear and while the rendering is in progress,
		// wait for the remaining updates to render.
		time.Sleep(time.Second)
		for progressWriter.IsRenderInProgress() {
			time.Sleep(progressUpdateFrequency)
		}
	}

	var uploadErrors []error
	for i := 0; i < len(completeEntries); i++ {
		if err := <-uploadErrChan; err != nil {
			uploadErrors = append(uploadErrors, err)
		}
	}

	if errCount := len(uploadErrors); errCount != 0 {
		fmt.Printf("\nFailed to upload %d worklog entries!\n\n", errCount)
		for _, err := range uploadErrors {
			fmt.Println(err)
		}
		os.Exit(1)
	}

	fmt.Printf("\nSuccessfully uploaded %d worklog entries!\n", len(completeEntries))
}

func Execute(buildVersion string, buildCommit string, buildDate string) {
	version = buildVersion
	commit = buildCommit
	date = buildDate

	cobra.CheckErr(rootCmd.Execute())
}
