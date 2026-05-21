/* 0044012f FUN_0044012f */

/* WARNING: Function: __security_check_cookie replaced with injection: security_check_cookie */

uint __cdecl FUN_0044012f(byte *param_1,byte *param_2)

{
  byte bVar1;
  _ptiddata p_Var2;
  int iVar3;
  byte *pbVar4;
  uint unaff_retaddr;
  byte local_28 [32];
  uint local_8;
  
  local_8 = DAT_00451a00 ^ unaff_retaddr;
  p_Var2 = __getptd();
  pbVar4 = local_28;
  for (iVar3 = 8; iVar3 != 0; iVar3 = iVar3 + -1) {
    pbVar4[0] = 0;
    pbVar4[1] = 0;
    pbVar4[2] = 0;
    pbVar4[3] = 0;
    pbVar4 = pbVar4 + 4;
  }
  do {
    bVar1 = *param_2;
    local_28[bVar1 >> 3] = local_28[bVar1 >> 3] | '\x01' << (bVar1 & 7);
    param_2 = param_2 + 1;
  } while (bVar1 != 0);
  if (param_1 == (byte *)0x0) {
    param_1 = (byte *)p_Var2->_token;
  }
  for (; (bVar1 = *param_1, pbVar4 = param_1, (local_28[bVar1 >> 3] & (byte)(1 << (bVar1 & 7))) != 0
         && (bVar1 != 0)); param_1 = param_1 + 1) {
  }
  do {
    if (*pbVar4 == 0) {
LAB_004401ca:
      p_Var2->_token = (char *)pbVar4;
      return -(uint)(param_1 != pbVar4) & (uint)param_1;
    }
    if ((local_28[*pbVar4 >> 3] & (byte)(1 << (*pbVar4 & 7))) != 0) {
      *pbVar4 = 0;
      pbVar4 = pbVar4 + 1;
      goto LAB_004401ca;
    }
    pbVar4 = pbVar4 + 1;
  } while( true );
}
