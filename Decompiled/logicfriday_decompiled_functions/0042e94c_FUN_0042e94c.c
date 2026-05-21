/* 0042e94c FUN_0042e94c */

/* WARNING: Function: __chkstk replaced with injection: alloca_probe */
/* WARNING: Function: __security_check_cookie replaced with injection: security_check_cookie */

void __fastcall FUN_0042e94c(int param_1)

{
  uint unaff_retaddr;
  int local_1418;
  uint local_1414 [257];
  int local_1010;
  uint local_100c [1025];
  uint local_8;
  
  local_8 = DAT_00451a00 ^ unaff_retaddr;
  local_1010 = 0;
  while( true ) {
    if (*(int *)(param_1 + 0x16c8) <= local_1010) goto LAB_0042edda;
    if ((*(int *)(*(int *)(*(int *)(param_1 + 0x16d0) + local_1010 * 4) + 0x44) != 0) &&
       (*(int *)(*(int *)(*(int *)(param_1 + 0x16d0) + local_1010 * 4) + 0x40) == 0)) break;
    local_1010 = local_1010 + 1;
  }
  FUN_0043ed39((char *)local_100c,
               (byte *)
               "Wire: %d\n\niTypeA = %d\niGateA = %d\niGateInA = %d\niWireA = %d\n\niTypeB = %d\niGateB = %d\niGateInB = %d\niWireB = %d\n\niIsOutput = %d\nbBroken = %d\nbRed = %d\nbSelected = %d\n\niNodeCnt = %d\n"
              );
  for (local_1418 = 0;
      local_1418 < *(int *)(*(int *)(*(int *)(param_1 + 0x16d0) + local_1010 * 4) + 0x28);
      local_1418 = local_1418 + 1) {
    FUN_0043ed39((char *)local_1414,(byte *)"pnode %d: {%d, %d}, iOrient=%d, bSel=%d, nID=%d\n");
    FUN_0043ebe0(local_100c,local_1414);
  }
  FUN_0043ed39((char *)local_1414,(byte *)"\niTapCnt = %d\n");
  FUN_0043ebe0(local_100c,local_1414);
  if (*(int *)(*(int *)(*(int *)(param_1 + 0x16d0) + local_1010 * 4) + 0x30) != 0) {
    for (local_1418 = 0;
        local_1418 < *(int *)(*(int *)(*(int *)(param_1 + 0x16d0) + local_1010 * 4) + 0x30);
        local_1418 = local_1418 + 1) {
      FUN_0043ed39((char *)local_1414,(byte *)"ptap %d: {%d, %d}, nID=%d, iTap=%d, iNode=%d\n");
      FUN_0043ebe0(local_100c,local_1414);
    }
  }
LAB_0042edda:
  MessageBoxA((HWND)0x0,(LPCSTR)local_100c,"",0);
  return;
}
