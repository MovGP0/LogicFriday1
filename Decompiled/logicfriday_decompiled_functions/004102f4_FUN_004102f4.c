/* 004102f4 FUN_004102f4 */

/* WARNING: Function: __security_check_cookie replaced with injection: security_check_cookie */

bool __thiscall FUN_004102f4(void *this,undefined4 param_1,undefined4 param_2,undefined4 param_3)

{
  BOOL BVar1;
  int iVar2;
  char *pcVar3;
  char *pcVar4;
  uint unaff_retaddr;
  char local_2c [36];
  uint local_8;
  
  local_8 = DAT_00451a00 ^ unaff_retaddr;
  pcVar3 = "Logic function (*.lfcn)";
  pcVar4 = local_2c;
  for (iVar2 = 8; iVar2 != 0; iVar2 = iVar2 + -1) {
    *(undefined4 *)pcVar4 = *(undefined4 *)pcVar3;
    pcVar3 = pcVar3 + 4;
    pcVar4 = pcVar4 + 4;
  }
  *pcVar4 = *pcVar3;
  *(undefined4 *)((int)this + 0x220) = param_1;
  *(undefined4 *)((int)this + 0x238) = param_2;
  *(undefined4 *)((int)this + 0x240) = param_3;
  *(int *)((int)this + 0x248) = (int)this + 0x270;
  *(undefined4 *)((int)this + 0x250) = 0x2004;
  *(char **)((int)this + 0x228) = local_2c;
  BVar1 = GetOpenFileNameA((LPOPENFILENAMEA)((int)this + 0x21c));
  if (BVar1 != 0) {
    lstrcpynA((LPSTR)((int)this + 0x270),*(LPCSTR *)((int)this + 0x238),
              *(ushort *)((int)this + 0x254) + 1);
  }
  return BVar1 != 0;
}
