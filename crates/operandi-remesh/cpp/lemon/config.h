/* LEMON configuration for Operandi's QuadriFlow build: what CMake writes
 * from lemon/config.h.in (Boost Software License 1.0, see
 * vendor/quadriflow/3rd/lemon-1.3.1/LICENSE) with no LP or MIP solver and
 * no threads, as in Blender's build. */
#define LEMON_VERSION "1.3.1"
#define LEMON_HAVE_LONG_LONG 1

#define _LEMON_CPLEX 1
#define _LEMON_CLP 2
#define _LEMON_GLPK 3
#define _LEMON_SOPLEX 4
#define _LEMON_CBC 5
