/* 00415864 FUN_00415864 */

void FUN_00415864(char *param_1,uint *param_2)

{
  char *pcVar1;
  
  pcVar1 = _strrchr(param_1,0x5c);
  if (pcVar1 == (char *)0x0) {
    FUN_0043ebd0(param_2,(uint *)&DAT_0044ba34);
  }
  else {
    lstrcpynA((LPSTR)param_2,param_1,(int)(pcVar1 + (1 - (int)param_1)));
  }
  return;
}
