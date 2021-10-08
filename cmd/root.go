package cmd

import (
	"context"
	"fmt"
	"github.com/gabor-boros/minutes/internal/pkg/client"
	"github.com/gabor-boros/minutes/internal/pkg/worklog"
	"github.com/spf13/cobra"
	"github.com/spf13/viper"
	"os"
	"regexp"
	"sort"
	"strings"
)

const (
	program            string = "minutes"
	worklogTableFormat string = "| %s\t| %s\t| %s\t| %s\t|\n"
)

var (
	configFile string
	envPrefix  string

	version string
	commit  string
	date    string

	sources = []string{"clockify", "tempo"}
	targets = []string{"tempo"}

	rootCmd = &cobra.Command{
		Use:   program,
		Short: "Sync worklogs between multiple time tracker, invoicing, and bookkeeping software.",
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
	rootCmd.Flags().StringP("date-format", "", "2006-01-02 15:04:05", "set start and end date format (in Go style)")

	rootCmd.Flags().StringP("source-user", "", "", "set the source user ID")
	rootCmd.Flags().StringP("source", "s", "", fmt.Sprintf("set the source of the sync %v", sources))

	rootCmd.Flags().StringP("target-user", "", "", "set the source user ID")
	rootCmd.Flags().StringP("target", "t", "", fmt.Sprintf("set the target of the sync %v", targets))

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

	if !isSliceContains(source, sources) {
		cobra.CheckErr(fmt.Sprintf("\"%s\" is not part of the supported sources %v\n", source, sources))
	}

	if !isSliceContains(target, targets) {
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
}

func runRootCmd(_ *cobra.Command, _ []string) {
	var err error

	if viper.GetBool("version") {
		fmt.Printf("%s version %s, commit %s (%s)\n", program, version, commit[:8], date)
		os.Exit(0)
	}

	validateFlags()

	start, err := getTime(viper.GetString("start"))
	cobra.CheckErr(err)

	end, err := getTime(viper.GetString("end"))
	cobra.CheckErr(err)

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

	sort.Slice(completeEntries, func(i int, j int) bool {
		return completeEntries[i].Task.Name < completeEntries[j].Task.Name
	})

	sort.Slice(incompleteEntries, func(i int, j int) bool {
		return incompleteEntries[i].Task.Name < incompleteEntries[j].Task.Name
	})

	printEntries("Incomplete entries", incompleteEntries)
	printEntries("Complete entries", completeEntries)

	if strings.ToLower(prompt("Continue? [y/n]")) != "y" {
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
