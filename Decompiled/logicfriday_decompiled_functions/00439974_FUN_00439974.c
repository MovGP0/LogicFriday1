/* 00439974 FUN_00439974 */

void __thiscall FUN_00439974(void *this,char *param_1)

{
  char *pcVar1;
  size_t sVar2;
  char *local_8;
  
  local_8 = param_1;
  pcVar1 = _strchr(param_1,0x3a);
  if (pcVar1 != (char *)0x0) {
    local_8 = _strchr(param_1,10);
    local_8 = local_8 + 1;
  }
  while( true ) {
    sVar2 = _strlen(local_8);
    if (local_8[sVar2 - 1] != '\r') break;
    local_8[sVar2 - 1] = '\0';
  }
  *(undefined4 *)((int)this + 0x28) = 0xdf0000;
  *(undefined4 *)((int)this + 0x18) = 0x40000000;
  SendMessageA(*(HWND *)((int)this + 4),0xb1,0xffffffff,-1);
  SendMessageA(*(HWND *)((int)this + 4),0xc2,0,(LPARAM)local_8);
  SendMessageA(*(HWND *)((int)this + 4),0x444,1,(int)this + 0x14);
  return;
}
