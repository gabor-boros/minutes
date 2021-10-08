package main

import (
	"github.com/gabor-boros/minutes/cmd"
)

var (
	version string
	commit  string
	date    string
)

func main() {
	cmd.Execute(version, commit, date)
}
