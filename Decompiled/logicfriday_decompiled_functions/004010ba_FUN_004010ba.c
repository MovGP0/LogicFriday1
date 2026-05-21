/* 004010ba FUN_004010ba */

char * __cdecl FUN_004010ba(int param_1,int param_2)

{
  uint local_8;
  
  local_8 = 0;
  do {
    if (1 < local_8) {
      return (char *)0x0;
    }
    if ((&DAT_00451060)[local_8 * 0xc6] == param_1) {
      if (param_2 == 0) {
        return s_logic_friday_sontrak_com_0045106c + local_8 * 0x318;
      }
      if (param_2 == 1) {
        *(undefined4 *)(&DAT_00451064 + local_8 * 0x318) = 1;
        return s_mailto_logic_friday_sontrak_com_00451170 + local_8 * 0x318;
      }
    }
    local_8 = local_8 + 1;
  } while( true );
}
