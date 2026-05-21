/* 00410476 FUN_00410476 */

/* WARNING: Function: __security_check_cookie replaced with injection: security_check_cookie */

bool __thiscall FUN_00410476(void *this,undefined4 param_1,undefined4 param_2,undefined4 param_3)

{
  BOOL BVar1;
  int iVar2;
  char *pcVar3;
  undefined4 *puVar4;
  uint unaff_retaddr;
  undefined4 local_24 [7];
  uint local_8;
  
  local_8 = DAT_00451a00 ^ unaff_retaddr;
  pcVar3 = "C Source File (*.c)";
  puVar4 = local_24;
  for (iVar2 = 6; iVar2 != 0; iVar2 = iVar2 + -1) {
    *puVar4 = *(undefined4 *)pcVar3;
    pcVar3 = pcVar3 + 4;
    puVar4 = puVar4 + 1;
  }
  *(undefined2 *)puVar4 = *(undefined2 *)pcVar3;
  *(undefined4 *)((int)this + 0x220) = param_1;
  *(undefined4 *)((int)this + 0x238) = param_2;
  *(undefined4 *)((int)this + 0x240) = param_3;
  *(int *)((int)this + 0x248) = (int)this + 0x478;
  *(undefined4 *)((int)this + 0x250) = 6;
  *(undefined4 **)((int)this + 0x228) = local_24;
  BVar1 = GetSaveFileNameA((LPOPENFILENAMEA)((int)this + 0x21c));
  if (BVar1 != 0) {
    lstrcpynA((LPSTR)((int)this + 0x478),*(LPCSTR *)((int)this + 0x238),
              *(ushort *)((int)this + 0x254) + 1);
  }
  return BVar1 != 0;
}
