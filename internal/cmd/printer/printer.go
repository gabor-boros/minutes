package printer

import (
	"fmt"
	"io"
	"strings"
	"time"

	"github.com/spf13/cobra"

	"github.com/gabor-boros/minutes/internal/cmd/utils"

	"github.com/gabor-boros/minutes/internal/pkg/worklog"
	"github.com/jedib0t/go-pretty/v6/table"
	"github.com/jedib0t/go-pretty/v6/text"
	"github.com/spf13/viper"
)

const (
	rowDateFormat    string = "2006-01-02 15:04:05"
	ColumnTask       string = "task"
	ColumnSummary    string = "summary"
	ColumnProject    string = "project"
	ColumnClient     string = "client"
	ColumnStart      string = "start"
	ColumnEnd        string = "end"
	ColumnBillable   string = "billable"
	ColumnUnbillable string = "unbillable"
)

// Columns lists all available columns that can be printed.
var Columns = []string{
	ColumnTask,
	ColumnSummary,
	ColumnProject,
	ColumnClient,
	ColumnStart,
	ColumnEnd,
	ColumnBillable,
	ColumnUnbillable,
}

// HideableColumns lists all columns that can be hidden when printing.
var HideableColumns = []string{
	ColumnSummary,
	ColumnProject,
	ColumnClient,
	ColumnStart,
	ColumnEnd,
}

// TableColumnConfig represents the configuration of a column.
// The configuration is built up from two parts, `Config` which stands for the
// table column config and `TruncateAt` which defines the max length a column
// text; longer texts will be truncated.
type TableColumnConfig struct {
	Config     table.ColumnConfig
	TruncateAt int
}

// Printer represents a printer that can write worklog entries.
type Printer interface {
	// Print prints out the list of complete and incomplete entries.
	// The output location must be set through `BasePrinterOpts`.
	Print(completeEntries []worklog.Entry, incompleteEntries []worklog.Entry) error
}

// BasePrinterOpts represents the configuration for common printer options.
type BasePrinterOpts struct {
	// Output is the location where `Print` prints.
	Output io.Writer
	// AutoIndex adds row number as the first column.
	AutoIndex bool
	// Title sets the printed data's title.
	// In case of tables, the title is the full-width first row.
	Title string
	// SortBy sets the list of columns that are used for sorting.
	// If a column name starts with `-` (hyphen), the direction is descending;
	// otherwise, the direction is treated as ascending.
	SortBy []string
	// HiddenColumns lists the columns that will be hidden during printing.
	HiddenColumns []string
}

// TablePrinterOpts represents the configuration for a table base printer.
// Table based printer sends the output to os.Stdout and draws an ascii-based
// table.
type TablePrinterOpts struct {
	BasePrinterOpts
	Style           table.Style
	ColumnConfig    []table.ColumnConfig
	ColumnTruncates map[string]int
}

type tablePrinter struct {
	writer      table.Writer
	truncateMap map[string]int
}

func (p *tablePrinter) convertEntryToRow(entry *worklog.Entry) table.Row {
	entryStart := entry.Start.Local()
	timeSpent := entry.BillableDuration + entry.UnbillableDuration

	return table.Row{
		utils.Truncate(entry.Task.Name, p.truncateMap[ColumnTask]),
		utils.Truncate(entry.Summary, p.truncateMap[ColumnSummary]),
		utils.Truncate(entry.Project.Name, p.truncateMap[ColumnProject]),
		utils.Truncate(entry.Client.Name, p.truncateMap[ColumnClient]),
		entryStart.Format(rowDateFormat),
		entryStart.Add(timeSpent).Format(rowDateFormat),
		entry.BillableDuration,
		entry.UnbillableDuration,
	}
}

func (p *tablePrinter) generateRows(entries []worklog.Entry, billable *time.Duration, unbillable *time.Duration) {
	for i := range entries {
		entry := entries[i]
		*billable += entry.BillableDuration
		*unbillable += entry.UnbillableDuration
		p.writer.AppendRow(p.convertEntryToRow(&entry))
	}
}

func (p *tablePrinter) Print(completeEntries []worklog.Entry, incompleteEntries []worklog.Entry) error {
	var totalBillable time.Duration
	var totalUnbillable time.Duration

	var header table.Row
	for _, column := range Columns {
		header = append(header, column)
	}

	p.writer.AppendHeader(header)

	p.generateRows(incompleteEntries, &totalBillable, &totalUnbillable)
	p.generateRows(completeEntries, &totalBillable, &totalUnbillable)

	p.writer.AppendFooter(table.Row{
		"", "", "", "", "", "total time spent", totalBillable.String(), totalUnbillable.String(),
	})
	p.writer.SetCaption(
		"You have %d complete and %d incomplete items. Before proceeding, please double-check them.\n",
		len(completeEntries),
		len(incompleteEntries),
	)
	p.writer.Render()

	return nil
}

// NewTablePrinter returns a new Printer that print tables to os.Stdout.
func NewTablePrinter(opts *TablePrinterOpts) Printer {
	writer := table.NewWriter()
	writer.SetOutputMirror(opts.Output)

	writer.SetTitle(opts.Title)
	writer.SetAutoIndex(opts.AutoIndex)

	writer.SetStyle(opts.Style)
	writer.Style().Format.Footer = text.FormatLower
	writer.SetColumnConfigs(opts.ColumnConfig)

	var sortBy []table.SortBy
	for _, column := range viper.GetStringSlice("table-sort-by") {
		mode := table.Asc

		if strings.HasPrefix(column, "-") {
			mode = table.Dsc
		}

		sortBy = append(sortBy, table.SortBy{
			Name: column,
			Mode: mode,
		})
	}

	writer.SortBy(sortBy)

	return &tablePrinter{
		writer:      writer,
		truncateMap: opts.ColumnTruncates,
	}
}

// ParseColumnConfigs parses the column configs taken from the config file.
// The hidden columns can be defined as flags and column config as well. During
// parsing, the flag based columns will take precedence.
func ParseColumnConfigs(key string, hiddenColumns []string) []table.ColumnConfig {
	var columnConfigs []table.ColumnConfig

	for _, column := range Columns {
		columnConfig := table.ColumnConfig{
			Name: column,
		}

		err := viper.UnmarshalKey(fmt.Sprintf(key, column), &columnConfig)
		cobra.CheckErr(err)

		if utils.IsSliceContains(column, hiddenColumns) {
			columnConfig.Hidden = true
		}

		columnConfigs = append(columnConfigs, columnConfig)
	}

	return columnConfigs
}
