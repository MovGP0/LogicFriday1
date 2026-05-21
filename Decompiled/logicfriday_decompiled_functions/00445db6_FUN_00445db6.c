/* 00445db6 FUN_00445db6 */

/* WARNING: Function: __SEH_prolog replaced with injection: SEH_prolog */
/* WARNING: Function: __SEH_epilog replaced with injection: EH_epilog3 */

void FUN_00445db6(void)

{
  undefined4 *local_20;
  
  for (local_20 = &DAT_0044ebac; local_20 < &DAT_0044ebac; local_20 = local_20 + 1) {
    if ((code *)*local_20 != (code *)0x0) {
      (*(code *)*local_20)();
    }
  }
  return;
}
