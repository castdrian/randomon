package main

import (
	"fmt"
	"os"
	"path/filepath"

	"github.com/castdrian/randomon/tools/internal/repo"
)

func main() {
	harness := repo.Path("tools", "vectors", "settings")
	upstream := repo.UpstreamSource()
	config := filepath.Join(upstream, "com", "dabomstew", "pkrandom", "config")
	classes := filepath.Join(harness, "build", "classes")
	work := filepath.Join(harness, "build", "work")
	output := repo.Path("crates", "randomon_settings", "tests", "vectors")
	if err := os.RemoveAll(filepath.Join(harness, "build")); err != nil {
		repo.Fail(err)
	}
	for _, directory := range []string{classes, work, output} {
		if err := os.MkdirAll(directory, 0o755); err != nil {
			repo.Fail(err)
		}
	}
	sources, err := repo.JavaSources(filepath.Join(harness, "src"))
	if err != nil {
		repo.Fail(err)
	}
	compile := append([]string{"-nowarn", "-encoding", "UTF-8", "-Xlint:none", "-sourcepath", upstream, "-d", classes}, sources...)
	if err := repo.Run(repo.Root(), "javac", compile...); err != nil {
		repo.Fail(err)
	}
	classPath := repo.ClassPath(classes, upstream)
	generate := func(separator string, arguments ...string) {
		command := append([]string{"-Djava.awt.headless=true", "-Dline.separator=" + separator, "-cp", classPath, "SettingsVectorGenerator"}, arguments...)
		if err := repo.Run(work, "java", command...); err != nil {
			repo.Fail(err)
		}
	}
	generate("\n", "all", output, config)
	generate("\r\n", "line-separator", output, config, "custom_names_crlf.json")
	generate("\n", "line-separator", output, config, "custom_names_lf.json")
	fmt.Println("Vectors written to", output)
}
