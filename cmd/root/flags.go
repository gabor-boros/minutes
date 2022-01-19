package root

import (
	"fmt"
	"regexp"
	"strings"

	"github.com/gabor-boros/minutes/internal/cmd/utils"
	"github.com/spf13/cobra"
	"github.com/spf13/viper"
)

var (
	sources = []string{"clockify", "harvest", "tempo", "timewarrior", "toggl"}
	targets = []string{"tempo"}
)

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

func initHarvestFlags() {
	rootCmd.Flags().StringP("harvest-api-key", "", "", "set the API key")
	rootCmd.Flags().IntP("harvest-account", "", 0, "set the Account ID")
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

	if source == "" {
		cobra.CheckErr("sync source must be set")
	}

	if target == "" {
		cobra.CheckErr("sync target must be set")
	}

	if source == target {
		cobra.CheckErr("sync source cannot match the target")
	}

	if !utils.IsSliceContains(source, sources) {
		cobra.CheckErr(fmt.Sprintf("\"%s\" is not part of the supported sources %v\n", source, sources))
	}

	if !utils.IsSliceContains(target, targets) {
		cobra.CheckErr(fmt.Sprintf("\"%s\" is not part of the supported targets %v\n", target, targets))
	}

	tagsAsTasksRegex := viper.GetString("tags-as-tasks-regex")
	_, err = regexp.Compile(tagsAsTasksRegex)
	cobra.CheckErr(err)

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
