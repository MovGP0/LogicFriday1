/* 0043f99d FUN_0043f99d */

/* WARNING: Function: __SEH_prolog replaced with injection: SEH_prolog */
/* WARNING: Function: __SEH_epilog replaced with injection: EH_epilog3 */

char * __cdecl FUN_0043f99d(char *param_1,int param_2,FILE *param_3)

{
  int *piVar1;
  uint uVar2;
  char *pcVar3;
  char *local_20;
  
  local_20 = param_1;
  if (param_2 < 1) {
    local_20 = (char *)0x0;
  }
  else {
    __lock_file(param_3);
    pcVar3 = param_1;
    do {
      param_2 = param_2 + -1;
      if (param_2 == 0) break;
      piVar1 = &param_3->_cnt;
      *piVar1 = *piVar1 + -1;
      if (*piVar1 < 0) {
        uVar2 = __filbuf(param_3);
      }
      else {
        uVar2 = (uint)(byte)*param_3->_ptr;
        param_3->_ptr = param_3->_ptr + 1;
      }
      if (uVar2 == 0xffffffff) {
        if (pcVar3 == param_1) {
          local_20 = (char *)0x0;
          goto LAB_0043fa06;
        }
        break;
      }
      *pcVar3 = (char)uVar2;
      pcVar3 = pcVar3 + 1;
    } while ((char)uVar2 != '\n');
    *pcVar3 = '\0';
LAB_0043fa06:
    FUN_0043fa1b();
  }
  return local_20;
}
