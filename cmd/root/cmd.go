package root

import (
	"context"
	"fmt"
	"os"
	"regexp"
	"strings"
	"time"

	"github.com/gabor-boros/minutes/internal/cmd/utils"

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
	initHarvestFlags()
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

	if err := viper.ReadInConfig(); err != nil {
		if _, ok := err.(viper.ConfigFileNotFoundError); !ok {
			cobra.CheckErr(err)
		}
	} else {
		fmt.Println("Using config file:", viper.ConfigFileUsed(), configFile)
	}

	// Bind flags to config value
	cobra.CheckErr(viper.BindPFlags(rootCmd.Flags()))
}

func runRootCmd(_ *cobra.Command, _ []string) {
	var err error

	if viper.GetBool("version") {
		if version == "" || len(commit) < 7 || date == "" {
			fmt.Println("dirty build")
		} else {
			fmt.Printf("%s version %s, commit %s (%s)\n", program, version, commit[:7], date)
		}
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

	tagsAsTasksRegex, err := regexp.Compile(viper.GetString("tags-as-tasks-regex"))
	cobra.CheckErr(err)

	entries, err := fetcher.FetchEntries(context.Background(), &client.FetchOpts{
		End:              end,
		Start:            start,
		User:             viper.GetString("source-user"),
		TagsAsTasksRegex: tagsAsTasksRegex,
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
