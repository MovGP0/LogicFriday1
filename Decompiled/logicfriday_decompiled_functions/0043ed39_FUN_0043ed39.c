/* 0043ed39 FUN_0043ed39 */

int __cdecl FUN_0043ed39(char *param_1,byte *param_2)

{
  int iVar1;
  FILE local_24;
  
  local_24._cnt = 0x7fffffff;
  local_24._flag = 0x42;
  local_24._base = param_1;
  local_24._ptr = param_1;
  iVar1 = FUN_00441127(&local_24,param_2,(wchar_t *)&stack0x0000000c);
  if (param_1 != (char *)0x0) {
    local_24._cnt = local_24._cnt + -1;
    if (local_24._cnt < 0) {
      __flsbuf(0,&local_24);
    }
    else {
      *local_24._ptr = '\0';
    }
  }
  return iVar1;
}
