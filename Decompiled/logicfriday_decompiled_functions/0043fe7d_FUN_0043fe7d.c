/* 0043fe7d FUN_0043fe7d */

char * __cdecl FUN_0043fe7d(int *param_1)

{
  int iVar1;
  int iVar2;
  _ptiddata p_Var3;
  char *pcVar4;
  int iVar5;
  char *pcVar6;
  char *pcVar7;
  int iVar8;
  
  p_Var3 = __getptd();
  if (p_Var3->_asctimebuf == (char *)0x0) {
    pcVar4 = _malloc(0x1a);
    p_Var3->_asctimebuf = pcVar4;
    pcVar6 = &DAT_0046c53c;
    if (pcVar4 == (char *)0x0) goto LAB_0043fea6;
  }
  pcVar6 = p_Var3->_asctimebuf;
LAB_0043fea6:
  iVar5 = param_1[6];
  iVar1 = param_1[4];
  iVar8 = 0;
  pcVar4 = pcVar6;
  do {
    pcVar7 = pcVar4;
    *pcVar7 = "SunMonTueWedThuFriSat"[iVar8 + iVar5 * 3];
    iVar2 = iVar1 * 3 + iVar8;
    iVar8 = iVar8 + 1;
    pcVar7[4] = "JanFebMarAprMayJunJulAugSepOctNovDec"[iVar2];
    pcVar4 = pcVar7 + 1;
  } while (iVar8 < 3);
  pcVar7[1] = ' ';
  pcVar7[5] = ' ';
  iVar5 = param_1[3];
  pcVar7[6] = (char)(iVar5 / 10) + '0';
  pcVar7[7] = (char)(iVar5 % 10) + '0';
  pcVar7[8] = ' ';
  iVar5 = param_1[2];
  pcVar7[9] = (char)(iVar5 / 10) + '0';
  pcVar7[10] = (char)(iVar5 % 10) + '0';
  pcVar7[0xb] = ':';
  iVar5 = param_1[1];
  pcVar7[0xc] = (char)(iVar5 / 10) + '0';
  pcVar7[0xd] = (char)(iVar5 % 10) + '0';
  pcVar7[0xe] = ':';
  iVar5 = *param_1;
  pcVar7[0xf] = (char)(iVar5 / 10) + '0';
  pcVar7[0x10] = (char)(iVar5 % 10) + '0';
  pcVar7[0x11] = ' ';
  iVar5 = param_1[5] / 100 + 0x13;
  pcVar7[0x12] = (char)(iVar5 / 10) + '0';
  pcVar7[0x13] = (char)(iVar5 % 10) + '0';
  iVar5 = param_1[5];
  pcVar7[0x14] = (char)((iVar5 % 100) / 10) + '0';
  pcVar7[0x15] = (char)((iVar5 % 100) % 10) + '0';
  pcVar7[0x16] = '\n';
  pcVar7[0x17] = '\0';
  return pcVar6;
}
