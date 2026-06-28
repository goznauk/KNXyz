from setuptools import Extension, setup
from Cython.Build import cythonize

import knxyz


# Build helper for the Cython cimport example.
setup(
    ext_modules=cythonize(
        [
            Extension(
                "knxyz_cython_smoke",
                ["knxyz_cython_smoke.pyx"],
                include_dirs=[knxyz.get_include()],
            )
        ],
        compiler_directives={"language_level": "3"},
    )
)
